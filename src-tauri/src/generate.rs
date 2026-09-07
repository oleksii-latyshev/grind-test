//! Prompt assembly and the three generation jobs.
//!
//! Prompts and schemas are embedded at compile time so a packaged app never depends on
//! files sitting next to the binary.

use anyhow::{anyhow, bail, Context, Result};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use crate::agy::{self, ModelTier};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::progress;
use crate::vault::quiz::{self, Difficulty, Question, QuestionKind, Quiz};
use crate::vault::syllabus::{self, Subject, Topic};
use crate::vault::Vault;

const KNOWLEDGE_PROMPT: &str = include_str!("../prompts/knowledge.md");
const QUIZ_PROMPT: &str = include_str!("../prompts/quiz.md");
const HINT_PROMPT: &str = include_str!("../prompts/hint.md");

const KNOWLEDGE_SCHEMA: &str = include_str!("../schemas/knowledge.json");
const QUIZ_SCHEMA: &str = include_str!("../schemas/quiz.json");
const HINT_SCHEMA: &str = include_str!("../schemas/hint.json");

const KNOWLEDGE_TIMEOUT: Duration = Duration::from_secs(420);
const QUIZ_TIMEOUT: Duration = Duration::from_secs(420);
const HINT_TIMEOUT: Duration = Duration::from_secs(180);

/// Per-topic slice of a knowledge note handed to the quiz generator.
const KNOWLEDGE_EXCERPT_CHARS: usize = 3500;
/// Backstop so a huge topic selection cannot overflow the process argument limit.
const MAX_PROMPT_CHARS: usize = 180_000;

// ---------------------------------------------------------------------------
// knowledge
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct KeyTerm {
    term: String,
    definition: String,
}

#[derive(Debug, Deserialize)]
struct KnowledgeDraft {
    summary: String,
    markdown: String,
    #[serde(default)]
    key_terms: Vec<KeyTerm>,
    #[serde(default)]
    exam_traps: Vec<String>,
}

impl KnowledgeDraft {
    /// Flatten the structured draft into the markdown body stored on disk.
    fn into_body(self) -> String {
        let mut body = String::new();
        body.push_str(self.summary.trim());
        body.push_str("\n\n");
        body.push_str(self.markdown.trim());

        if !self.key_terms.is_empty() {
            body.push_str("\n\n## Ключові терміни\n\n");
            for term in &self.key_terms {
                body.push_str(&format!("- **{}** — {}\n", term.term.trim(), term.definition.trim()));
            }
        }
        if !self.exam_traps.is_empty() {
            body.push_str("\n## На чому підловлюють\n\n");
            for trap in &self.exam_traps {
                body.push_str(&format!("- {}\n", trap.trim()));
            }
        }
        body
    }
}

fn knowledge_prompt(subject: &Subject, topic: &Topic) -> String {
    let siblings = subject
        .sections
        .iter()
        .find(|s| s.index == topic.section)
        .map(|section| {
            section
                .topics
                .iter()
                .filter(|t| t.id != topic.id)
                .map(|t| format!("- {}", t.title))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    KNOWLEDGE_PROMPT
        .replace("{{SUBJECT_TITLE}}", &subject.title)
        .replace("{{SECTION_TITLE}}", &topic.section_title)
        .replace("{{TOPIC_TITLE}}", &topic.title)
        .replace(
            "{{SIBLING_TOPICS}}",
            if siblings.is_empty() { "(none)" } else { &siblings },
        )
}

/// Generate and persist the note for a single topic.
pub async fn knowledge_for_topic(
    vault: &Vault,
    subject: &Subject,
    topic: &Topic,
) -> Result<(KnowledgeNote, agy::Usage)> {
    let prompt = knowledge_prompt(subject, topic);
    let response = agy::run::<KnowledgeDraft>(
        &prompt,
        ModelTier::Smart,
        KNOWLEDGE_SCHEMA,
        KNOWLEDGE_TIMEOUT,
    )
    .await
    .with_context(|| format!("generating knowledge for {}", topic.id))?;

    let body = response.data.into_body();
    if body.trim().len() < 200 {
        bail!("model returned a suspiciously short note for {}", topic.id);
    }
    let note = knowledge::write(vault, topic, &response.model, &body)?;
    Ok((note, response.usage))
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenerationEvent {
    Started { subject: String, total: usize, skipped: usize },
    TopicStarted { topic_id: String, title: String },
    TopicDone { topic_id: String, title: String, done: usize, total: usize },
    TopicFailed { topic_id: String, title: String, error: String },
    Finished { generated: usize, failed: usize, total_tokens: u64 },
}

pub type EventSink = Arc<dyn Fn(GenerationEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeBatchReport {
    pub generated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_tokens: u64,
    pub errors: Vec<String>,
}

/// Fill in missing knowledge notes for a subject.
///
/// Every note is written the moment it lands, and topics that already have one are skipped,
/// so an interrupted run costs nothing but the calls that were in flight.
pub async fn knowledge_batch(
    vault: &Vault,
    subject_id: &str,
    topic_ids: Option<Vec<String>>,
    force: bool,
    concurrency: usize,
    on_event: EventSink,
) -> Result<KnowledgeBatchReport> {
    let subject = syllabus::load(vault, subject_id)?;

    let selected: Vec<Topic> = match &topic_ids {
        Some(ids) => subject
            .topics()
            .filter(|t| ids.contains(&t.id))
            .cloned()
            .collect(),
        None => subject.topics().cloned().collect(),
    };

    let total_selected = selected.len();
    let pending: Vec<Topic> = selected
        .into_iter()
        .filter(|topic| force || !knowledge::exists(vault, topic))
        .collect();
    let skipped = total_selected - pending.len();

    on_event(GenerationEvent::Started {
        subject: subject.id.clone(),
        total: pending.len(),
        skipped,
    });

    let total = pending.len();
    let subject = Arc::new(subject);
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let results: Vec<Result<u64, (Topic, String)>> = stream::iter(pending.into_iter().map(|topic| {
        let subject = Arc::clone(&subject);
        let on_event = Arc::clone(&on_event);
        let done = Arc::clone(&done);
        let vault = vault.clone();
        async move {
            on_event(GenerationEvent::TopicStarted {
                topic_id: topic.id.clone(),
                title: topic.title.clone(),
            });
            match knowledge_for_topic(&vault, &subject, &topic).await {
                Ok((_, usage)) => {
                    let position = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    on_event(GenerationEvent::TopicDone {
                        topic_id: topic.id.clone(),
                        title: topic.title.clone(),
                        done: position,
                        total,
                    });
                    Ok(usage.total_tokens)
                }
                Err(error) => {
                    let message = format!("{error:#}");
                    on_event(GenerationEvent::TopicFailed {
                        topic_id: topic.id.clone(),
                        title: topic.title.clone(),
                        error: message.clone(),
                    });
                    Err((topic, message))
                }
            }
        }
    }))
    .buffer_unordered(concurrency.max(1))
    .collect()
    .await;

    let mut report = KnowledgeBatchReport {
        generated: 0,
        skipped,
        failed: 0,
        total_tokens: 0,
        errors: Vec::new(),
    };
    for result in results {
        match result {
            Ok(tokens) => {
                report.generated += 1;
                report.total_tokens += tokens;
            }
            Err((topic, message)) => {
                report.failed += 1;
                report.errors.push(format!("{}: {}", topic.id, message));
            }
        }
    }

    on_event(GenerationEvent::Finished {
        generated: report.generated,
        failed: report.failed,
        total_tokens: report.total_tokens,
    });
    Ok(report)
}

// ---------------------------------------------------------------------------
// quiz
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct QuestionDraft {
    topic_id: String,
    #[serde(rename = "type")]
    kind: QuestionKind,
    question: String,
    options: Vec<String>,
    correct: Vec<usize>,
    explanation: String,
    difficulty: DraftDifficulty,
}

/// The generator only ever labels a question easy/medium/hard; `mixed` is a request, not a
/// property of a question.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DraftDifficulty {
    Easy,
    Medium,
    Hard,
}

impl From<DraftDifficulty> for Difficulty {
    fn from(value: DraftDifficulty) -> Self {
        match value {
            DraftDifficulty::Easy => Difficulty::Easy,
            DraftDifficulty::Medium => Difficulty::Medium,
            DraftDifficulty::Hard => Difficulty::Hard,
        }
    }
}

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
        bail!("no topics matched the selection for subject '{}'", request.subject);
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
        id: format!("{}-{}", now.format("%Y%m%d-%H%M%S"), &uuid::Uuid::new_v4().to_string()[..8]),
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

/// Drop questions the model got structurally wrong rather than shipping a quiz that cannot
/// be answered correctly.
fn sanitize_questions(drafts: Vec<QuestionDraft>, topic_ids: &[String]) -> Vec<Question> {
    let fallback = topic_ids.first().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for (index, draft) in drafts.into_iter().enumerate() {
        if draft.options.len() < 2 || draft.question.trim().is_empty() {
            continue;
        }
        let mut correct: Vec<usize> = draft
            .correct
            .into_iter()
            .filter(|i| *i < draft.options.len())
            .collect();
        correct.sort_unstable();
        correct.dedup();
        if correct.is_empty() || correct.len() == draft.options.len() {
            continue;
        }
        // Trust the shape of the answer over the model's own `type` label.
        let kind = if correct.len() > 1 {
            QuestionKind::Multi
        } else {
            QuestionKind::Single
        };
        let _ = draft.kind;

        out.push(Question {
            id: format!("q{}", index + 1),
            topic_id: if topic_ids.contains(&draft.topic_id) {
                draft.topic_id
            } else {
                fallback.clone()
            },
            kind,
            question: draft.question.trim().to_string(),
            options: draft.options.into_iter().map(|o| o.trim().to_string()).collect(),
            correct,
            explanation: draft.explanation.trim().to_string(),
            difficulty: draft.difficulty.into(),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// hint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hint {
    pub hint: String,
    #[serde(default)]
    pub related_concepts: Vec<String>,
}

pub async fn hint(vault: &Vault, subject_id: &str, quiz_id: &str, question_id: &str) -> Result<Hint> {
    let quiz = quiz::read(vault, subject_id, quiz_id)?;
    let question = quiz
        .questions
        .iter()
        .find(|q| q.id == question_id)
        .ok_or_else(|| anyhow!("question '{question_id}' not found in quiz '{quiz_id}'"))?;

    let subject = syllabus::load(vault, subject_id)?;
    let note = subject
        .find_topic(&question.topic_id)
        .map(|topic| knowledge::read(vault, topic))
        .transpose()?
        .flatten();

    let knowledge_block = match note {
        Some(note) => truncate_chars(&note.body, 6000),
        None => "(конспект для цієї теми ще не згенеровано)".to_string(),
    };

    let options = question
        .options
        .iter()
        .enumerate()
        .map(|(i, option)| format!("{}. {}", i + 1, option))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = HINT_PROMPT
        .replace("{{QUESTION}}", &question.question)
        .replace("{{OPTIONS}}", &options)
        .replace("{{KNOWLEDGE}}", &knowledge_block);

    let response = agy::run::<Hint>(&prompt, ModelTier::Smart, HINT_SCHEMA, HINT_TIMEOUT).await?;
    Ok(response.data)
}

// ---------------------------------------------------------------------------

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

fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_string(),
        Some((cut, _)) => format!("{}\n…", &text[..cut]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(options: Vec<&str>, correct: Vec<usize>) -> QuestionDraft {
        QuestionDraft {
            topic_id: "f3/1.1".into(),
            kind: QuestionKind::Single,
            question: "Питання?".into(),
            options: options.into_iter().map(String::from).collect(),
            correct,
            explanation: "бо так".into(),
            difficulty: DraftDifficulty::Medium,
        }
    }

    #[test]
    fn drops_unanswerable_questions() {
        let topics = vec!["f3/1.1".to_string()];
        let kept = sanitize_questions(
            vec![
                draft(vec!["a", "b", "c", "d"], vec![1]),
                draft(vec!["a", "b", "c", "d"], vec![]),          // no correct option
                draft(vec!["a", "b", "c", "d"], vec![0, 1, 2, 3]), // every option correct
                draft(vec!["a"], vec![0]),                         // not a choice
                draft(vec!["a", "b", "c", "d"], vec![9]),          // out of range
            ],
            &topics,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "q1");
    }

    #[test]
    fn infers_kind_from_the_answer_shape() {
        let topics = vec!["f3/1.1".to_string()];
        let kept = sanitize_questions(vec![draft(vec!["a", "b", "c", "d"], vec![0, 2])], &topics);
        assert!(matches!(kept[0].kind, QuestionKind::Multi));
    }

    #[test]
    fn unknown_topic_ids_fall_back_to_a_real_one() {
        let topics = vec!["f3/1.1".to_string()];
        let mut d = draft(vec!["a", "b", "c", "d"], vec![0]);
        d.topic_id = "made/up".into();
        let kept = sanitize_questions(vec![d], &topics);
        assert_eq!(kept[0].topic_id, "f3/1.1");
    }
}
