//! Practice quizzes at the fast tier, grounded in the notes of the topics they sample.

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

use super::questions::{sanitize_questions, QuestionDraft};
use super::{truncate_chars, QUIZ_TIMEOUT};
use crate::agy::{self, ModelTier};
use crate::vault::quiz::{self, Difficulty, Quiz};
use crate::vault::syllabus::{self, Topic};
use crate::vault::{knowledge, progress, Vault};

const QUIZ_PROMPT: &str = include_str!("../../prompts/quiz.md");
const QUIZ_SCHEMA: &str = include_str!("../../schemas/quiz.json");

/// Per-topic slice of a knowledge note handed to the quiz generator.
const KNOWLEDGE_EXCERPT_CHARS: usize = 3500;
/// Backstop so a huge topic selection cannot overflow the process argument limit.
const MAX_PROMPT_CHARS: usize = 180_000;

#[derive(Debug, Deserialize)]
struct QuizDraft {
    questions: Vec<QuestionDraft>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuizRequest {
    pub subject: String,
    /// Explicit topic selection; when absent, topics are sampled by weakness.
    #[serde(default)]
    pub topic_ids: Option<Vec<String>>,
    pub question_count: usize,
    pub difficulty: Difficulty,
}

pub async fn quiz(vault: &Vault, request: QuizRequest) -> Result<Quiz> {
    let subject = syllabus::load(vault, &request.subject)?;
    let mastery = progress::load_mastery(vault);

    let all: Vec<&Topic> = subject.topics().collect();
    let candidates: Vec<&Topic> = match &request.topic_ids {
        Some(ids) if !ids.is_empty() => all
            .iter()
            .copied()
            .filter(|t| ids.contains(&t.id))
            .collect(),
        _ => all.clone(),
    };
    if candidates.is_empty() {
        bail!(
            "no topics matched the selection for subject '{}'",
            request.subject
        );
    }

    // Only topics with a knowledge note can be quizzed — questions must be grounded.
    let grounded: Vec<&Topic> = candidates
        .into_iter()
        .filter(|topic| knowledge::exists(vault, topic))
        .collect();
    if grounded.is_empty() {
        bail!(
            "no knowledge notes exist for the selected topics — generate knowledge for '{}' first",
            request.subject
        );
    }

    // Roughly two questions per topic keeps each note actually used rather than skimmed.
    let topic_budget = ((request.question_count + 1) / 2).clamp(1, 12);
    let chosen = progress::pick_topics(&mastery, &grounded, topic_budget);

    let mut knowledge_block = String::new();
    let mut topic_ids = Vec::new();
    for topic in &chosen {
        let Some(note) = knowledge::read(vault, topic)? else {
            continue;
        };
        let excerpt = truncate_chars(&note.body, KNOWLEDGE_EXCERPT_CHARS);
        let section = format!(
            "### topic_id: {}\nТема: {}\n\n{}\n\n",
            topic.id, topic.title, excerpt
        );
        if knowledge_block.len() + section.len() > MAX_PROMPT_CHARS {
            break;
        }
        knowledge_block.push_str(&section);
        topic_ids.push(topic.id.clone());
    }
    if topic_ids.is_empty() {
        bail!("could not load any knowledge notes for the selected topics");
    }

    let weak: Vec<String> = chosen
        .iter()
        .filter(|topic| {
            mastery
                .topics
                .get(&topic.id)
                .is_some_and(|m| m.seen > 0 && m.accuracy() < 0.7)
        })
        .map(|topic| format!("- {} ({})", topic.title, topic.id))
        .collect();

    let recent = quiz::recent_question_texts(vault, &request.subject, 40);

    let weak_block = if weak.is_empty() {
        "(none yet)".to_string()
    } else {
        weak.join("\n")
    };
    let recent_block = if recent.is_empty() {
        "(none yet)".to_string()
    } else {
        recent
            .iter()
            .map(|question| format!("- {question}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = QUIZ_PROMPT
        .replace("{{QUESTION_COUNT}}", &request.question_count.to_string())
        .replace("{{KNOWLEDGE}}", &knowledge_block)
        .replace("{{DIFFICULTY}}", request.difficulty.as_prompt())
        .replace("{{WEAK_TOPICS}}", &weak_block)
        .replace("{{RECENT_QUESTIONS}}", &recent_block);

    let response =
        agy::run::<QuizDraft>(&prompt, ModelTier::Fast, QUIZ_SCHEMA, QUIZ_TIMEOUT).await?;

    let questions = sanitize_questions(response.data.questions, &topic_ids);
    if questions.is_empty() {
        bail!("the generator returned no usable questions");
    }

    let now = chrono::Utc::now();
    let quiz = Quiz {
        id: format!(
            "{}-{}",
            now.format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        ),
        subject: request.subject.clone(),
        title: format!("{} — {} питань", subject.title, questions.len()),
        created_at: now.to_rfc3339(),
        model: response.model,
        difficulty: request.difficulty,
        topic_ids,
        questions,
    };
    quiz::save(vault, &quiz)?;
    Ok(quiz)
}

pub fn grade(
    vault: &Vault,
    subject_id: &str,
    quiz_id: &str,
    selections: BTreeMap<String, Vec<usize>>,
    hints_used: Vec<String>,
) -> Result<progress::AttemptResult> {
    let quiz = quiz::read(vault, subject_id, quiz_id)?;
    progress::record_attempt(vault, &quiz, &selections, &hints_used)
}
