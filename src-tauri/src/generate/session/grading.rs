//! Grading a finished session in one smart call and moving each topic along the ladder.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::plan::SessionPlan;
use super::store::{load_session, save_session};
use crate::agy::{self, ModelTier};
use crate::generate::{truncate_chars, KNOWLEDGE_TIMEOUT};
use crate::vault::{progress, study, Vault};

const GRADE_OPEN_PROMPT: &str = include_str!("../../../prompts/grade_open.md");
const GRADE_OPEN_SCHEMA: &str = include_str!("../../../schemas/grade_open.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenGrading {
    pub topic_id: String,
    pub score: u32,
    pub verdict: String,
    #[serde(default)]
    pub covered: Vec<String>,
    #[serde(default)]
    pub missed: Vec<String>,
    #[serde(default)]
    pub correction: String,
}

#[derive(Debug, Deserialize)]
struct GradingDraft {
    gradings: Vec<OpenGrading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicOutcome {
    pub topic_id: String,
    pub title: String,
    pub score: u32,
    pub open_score: Option<u32>,
    pub quiz_correct: usize,
    pub quiz_total: usize,
    pub level: u8,
    pub stage: study::Stage,
    pub due_at: Option<String>,
    /// Both halves left blank: the topic was read but not scored, and its schedule untouched.
    #[serde(default)]
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub session_id: String,
    pub subject: String,
    pub gradings: Vec<OpenGrading>,
    pub answers: Vec<progress::AnswerRecord>,
    pub topics: Vec<TopicOutcome>,
    pub overall_score: u32,
}

/// The written answer imitates the exam, so it weighs more than the multiple-choice half.
const OPEN_WEIGHT: f64 = 0.6;

/// Combine the two halves of a topic's score. A half the student skipped does not count, and
/// with both skipped there is no score at all — skipping is not evidence of not knowing.
fn topic_score(open: Option<u32>, quiz: Option<u32>) -> Option<u32> {
    match (open, quiz) {
        (Some(open), Some(quiz)) => {
            Some((open as f64 * OPEN_WEIGHT + quiz as f64 * (1.0 - OPEN_WEIGHT)).round() as u32)
        }
        (Some(score), None) | (None, Some(score)) => Some(score),
        (None, None) => None,
    }
}

/// Grade both halves of a session, update mastery and the review schedule, and persist the
/// whole thing.
pub async fn finish_session(
    vault: &Vault,
    session_id: &str,
    subject_id: &str,
    open_answers: BTreeMap<String, String>,
    quiz_selections: BTreeMap<String, Vec<usize>>,
) -> Result<SessionResult> {
    let plan = load_session(vault, subject_id, session_id)?;

    // Blank answers never reach the grader: they would only come back as 0, and a session
    // with nothing written costs no smart call at all.
    let open_answers: BTreeMap<String, String> = open_answers
        .into_iter()
        .filter(|(topic_id, answer)| {
            !answer.trim().is_empty() && plan.open_questions.iter().any(|q| &q.topic_id == topic_id)
        })
        .collect();

    let gradings = if open_answers.is_empty() {
        Vec::new()
    } else {
        grade_open(&plan, &open_answers).await?
    };

    let answers: Vec<progress::AnswerRecord> = plan
        .quiz
        .iter()
        .map(|question| {
            let selected = quiz_selections
                .get(&question.id)
                .cloned()
                .unwrap_or_default();
            progress::AnswerRecord {
                question_id: question.id.clone(),
                topic_id: question.topic_id.clone(),
                correct: question.is_correct(&selected),
                selected,
                hint_used: false,
            }
        })
        .collect();
    // Unanswered questions stay in the review below but are not recorded as mistakes.
    let attempted: Vec<progress::AnswerRecord> = answers
        .iter()
        .filter(|answer| !answer.selected.is_empty())
        .cloned()
        .collect();
    progress::apply_answers(vault, &attempted)?;

    let mut scores = BTreeMap::new();
    let mut outcomes = Vec::new();
    for entry in &plan.topics {
        let topic_id = &entry.topic.id;
        let open_score = gradings
            .iter()
            .find(|grading| &grading.topic_id == topic_id)
            .map(|grading| grading.score.min(100));

        let topic_answers: Vec<&progress::AnswerRecord> =
            answers.iter().filter(|a| &a.topic_id == topic_id).collect();
        let quiz_total = topic_answers.len();
        let quiz_correct = topic_answers.iter().filter(|a| a.correct).count();
        // Not one of the topic's questions touched: the quiz half was skipped, not failed.
        let quiz_score = topic_answers
            .iter()
            .any(|answer| !answer.selected.is_empty())
            .then(|| ((quiz_correct as f64 / quiz_total as f64) * 100.0).round() as u32);

        let score = topic_score(open_score, quiz_score);
        if let Some(score) = score {
            scores.insert(topic_id.clone(), score);
        }
        outcomes.push(TopicOutcome {
            topic_id: topic_id.clone(),
            title: entry.topic.title.clone(),
            score: score.unwrap_or(0),
            open_score,
            quiz_correct,
            quiz_total,
            level: 0,
            stage: study::Stage::New,
            due_at: None,
            skipped: score.is_none(),
        });
    }

    // Scheduling is the source of truth for the level a topic lands on, so read it back
    // rather than recomputing it here.
    let state = study::record_scores(vault, &scores)?;
    for outcome in &mut outcomes {
        if let Some(entry) = state.topics.get(&outcome.topic_id) {
            outcome.level = entry.level;
            outcome.stage = entry.stage();
            outcome.due_at = entry.due_at.clone();
        }
    }

    let scored: Vec<f64> = outcomes
        .iter()
        .filter(|outcome| !outcome.skipped)
        .map(|outcome| outcome.score as f64)
        .collect();
    let overall_score = if scored.is_empty() {
        0
    } else {
        (scored.iter().sum::<f64>() / scored.len() as f64).round() as u32
    };

    let result = SessionResult {
        session_id: plan.id.clone(),
        subject: plan.subject.clone(),
        gradings,
        answers,
        topics: outcomes,
        overall_score,
    };
    save_session(vault, &plan, Some(&result))?;
    Ok(result)
}

async fn grade_open(
    plan: &SessionPlan,
    open_answers: &BTreeMap<String, String>,
) -> Result<Vec<OpenGrading>> {
    let mut block = String::new();
    // Only the answers actually written: `finish_session` has already dropped blank ones.
    let answered = plan
        .open_questions
        .iter()
        .filter_map(|question| Some((question, open_answers.get(&question.topic_id)?.trim())));
    for (index, (question, answer)) in answered.enumerate() {
        let note = plan
            .notes
            .iter()
            .find(|note| note.topic_id == question.topic_id);

        block.push_str(&format!(
            "## Відповідь {} — topic_id: {}\n\n### Питання\n{}\n\n### Очікувані пункти\n{}\n\n### Відповідь студента\n{}\n\n### Конспект теми\n{}\n\n",
            index + 1,
            question.topic_id,
            question.question,
            question
                .expected_points
                .iter()
                .map(|point| format!("- {point}"))
                .collect::<Vec<_>>()
                .join("\n"),
            answer,
            note.map(|note| truncate_chars(&note.body, 5000))
                .unwrap_or_else(|| "(конспект недоступний)".to_string()),
        ));
    }

    let prompt = GRADE_OPEN_PROMPT
        .replace("{{ANSWER_STYLE}}", plan.mode.answer_style())
        .replace("{{ANSWERS}}", &block);
    let response = agy::run::<GradingDraft>(
        &prompt,
        ModelTier::Smart,
        GRADE_OPEN_SCHEMA,
        KNOWLEDGE_TIMEOUT,
    )
    .await?;

    Ok(response
        .data
        .gradings
        .into_iter()
        .filter(|grading| open_answers.contains_key(&grading.topic_id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skipped_half_does_not_count_as_zero() {
        assert_eq!(topic_score(Some(80), Some(50)), Some(68));
        assert_eq!(topic_score(None, Some(50)), Some(50));
        assert_eq!(topic_score(Some(80), None), Some(80));
        assert_eq!(topic_score(None, None), None);
    }
}
