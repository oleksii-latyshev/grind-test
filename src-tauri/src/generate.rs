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
use crate::vault::study;
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
                body.push_str(&format!(
                    "- **{}** — {}\n",
                    term.term.trim(),
                    term.definition.trim()
                ));
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
            if siblings.is_empty() {
                "(none)"
            } else {
                &siblings
            },
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
    Started {
        subject: String,
        total: usize,
        skipped: usize,
    },
    TopicStarted {
        topic_id: String,
        title: String,
    },
    TopicDone {
        topic_id: String,
        title: String,
        done: usize,
        total: usize,
    },
    TopicFailed {
        topic_id: String,
        title: String,
        error: String,
    },
    Finished {
        generated: usize,
        failed: usize,
        total_tokens: u64,
    },
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

    let results: Vec<Result<u64, (Topic, String)>> =
        stream::iter(pending.into_iter().map(|topic| {
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
            options: draft
                .options
                .into_iter()
                .map(|o| o.trim().to_string())
                .collect(),
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

pub async fn hint(
    vault: &Vault,
    subject_id: &str,
    quiz_id: &str,
    question_id: &str,
) -> Result<Hint> {
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
                draft(vec!["a", "b", "c", "d"], vec![]), // no correct option
                draft(vec!["a", "b", "c", "d"], vec![0, 1, 2, 3]), // every option correct
                draft(vec!["a"], vec![0]),               // not a choice
                draft(vec!["a", "b", "c", "d"], vec![9]), // out of range
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

// ---------------------------------------------------------------------------
// study sessions
// ---------------------------------------------------------------------------

const SESSION_PROMPT: &str = include_str!("../prompts/session.md");
const GRADE_OPEN_PROMPT: &str = include_str!("../prompts/grade_open.md");
const SESSION_SCHEMA: &str = include_str!("../schemas/session.json");
const GRADE_OPEN_SCHEMA: &str = include_str!("../schemas/grade_open.json");

/// Notes are quoted in full for a session: the questions must not test material the
/// student was never shown.
const SESSION_EXCERPT_CHARS: usize = 9000;

/// Beyond this the single generation call has to cover too much ground, and the reading
/// stretch stops being one sitting.
pub const MAX_SESSION_TOPICS: usize = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub topic_id: String,
    pub question: String,
    /// The grading rubric: what a complete answer has to contain.
    pub expected_points: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SessionDraft {
    open_questions: Vec<OpenQuestionDraft>,
    quiz_questions: Vec<QuestionDraft>,
}

#[derive(Debug, Deserialize)]
struct OpenQuestionDraft {
    topic_id: String,
    question: String,
    #[serde(default)]
    expected_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPlan {
    pub id: String,
    pub subject: String,
    pub created_at: String,
    pub topics: Vec<study::PlannedTopic>,
    pub notes: Vec<KnowledgeNote>,
    pub open_questions: Vec<OpenQuestion>,
    pub quiz: Vec<Question>,
}

/// Build a session: pick topics, load their notes, and generate the open questions and the
/// mini-quiz in a single fast-model call.
///
/// `topic_ids` overrides the scheduler — the student can insist on the topics they know are
/// coming up, rather than the ones the ladder would serve next.
pub async fn start_session(
    vault: &Vault,
    subject_id: &str,
    size: usize,
    topic_ids: Option<Vec<String>>,
) -> Result<SessionPlan> {
    let subject = syllabus::load(vault, subject_id)?;
    let state = study::load(vault);

    let studiable: Vec<&Topic> = subject
        .topics()
        .filter(|topic| knowledge::exists(vault, topic))
        .collect();
    if studiable.is_empty() {
        bail!("no knowledge notes exist for '{subject_id}' yet — generate them before studying");
    }

    let planned = match topic_ids {
        Some(ids) if !ids.is_empty() => {
            let chosen: Vec<study::PlannedTopic> = studiable
                .iter()
                .filter(|topic| ids.contains(&topic.id))
                .take(MAX_SESSION_TOPICS)
                .map(|topic| study::planned_for(&state, topic))
                .collect();
            if chosen.is_empty() {
                bail!("none of the chosen topics has a knowledge note yet");
            }
            chosen
        }
        _ => study::plan_session(&state, &studiable, size.clamp(1, MAX_SESSION_TOPICS)),
    };
    if planned.is_empty() {
        bail!("nothing is due right now — every topic in this subject is scheduled for later");
    }

    let mut notes = Vec::new();
    let mut knowledge_block = String::new();
    for entry in &planned {
        let Some(note) = knowledge::read(vault, &entry.topic)? else {
            continue;
        };
        knowledge_block.push_str(&format!(
            "### topic_id: {}\nТема: {}\n\n{}\n\n",
            entry.topic.id,
            entry.topic.title,
            truncate_chars(&note.body, SESSION_EXCERPT_CHARS)
        ));
        notes.push(note);
    }
    if notes.is_empty() {
        bail!("could not load the knowledge notes for the planned topics");
    }

    let quiz_count = (notes.len() * 4).clamp(6, 20);
    let prompt = SESSION_PROMPT
        .replace("{{KNOWLEDGE}}", &knowledge_block)
        .replace("{{QUIZ_COUNT}}", &quiz_count.to_string());

    let response =
        agy::run::<SessionDraft>(&prompt, ModelTier::Fast, SESSION_SCHEMA, QUIZ_TIMEOUT).await?;

    let topic_ids: Vec<String> = notes.iter().map(|note| note.topic_id.clone()).collect();
    let quiz = sanitize_questions(response.data.quiz_questions, &topic_ids);
    if quiz.is_empty() {
        bail!("the generator returned no usable quiz questions for this session");
    }

    // One open question per topic, in the order the student read them; anything the model
    // invented for an unknown topic is dropped.
    let open_questions: Vec<OpenQuestion> = topic_ids
        .iter()
        .filter_map(|topic_id| {
            response
                .data
                .open_questions
                .iter()
                .find(|draft| &draft.topic_id == topic_id)
                .map(|draft| OpenQuestion {
                    topic_id: draft.topic_id.clone(),
                    question: draft.question.trim().to_string(),
                    expected_points: draft.expected_points.clone(),
                })
        })
        .collect();

    let now = chrono::Utc::now();
    let plan = SessionPlan {
        id: format!(
            "{}-{}",
            now.format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        ),
        subject: subject_id.to_string(),
        created_at: now.to_rfc3339(),
        topics: planned
            .into_iter()
            .filter(|entry| topic_ids.contains(&entry.topic.id))
            .collect(),
        notes,
        open_questions,
        quiz,
    };
    save_session(vault, &plan, None)?;
    Ok(plan)
}

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

    let gradings = if plan.open_questions.is_empty() {
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
    progress::apply_answers(vault, &answers)?;

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
        let quiz_score = (quiz_total > 0)
            .then(|| ((quiz_correct as f64 / quiz_total as f64) * 100.0).round() as u32);

        let score = match (open_score, quiz_score) {
            (Some(open), Some(quiz)) => {
                (open as f64 * OPEN_WEIGHT + quiz as f64 * (1.0 - OPEN_WEIGHT)).round() as u32
            }
            (Some(open), None) => open,
            (None, Some(quiz)) => quiz,
            (None, None) => 0,
        };

        scores.insert(topic_id.clone(), score);
        outcomes.push(TopicOutcome {
            topic_id: topic_id.clone(),
            title: entry.topic.title.clone(),
            score,
            open_score,
            quiz_correct,
            quiz_total,
            level: 0,
            stage: study::Stage::New,
            due_at: None,
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

    let overall_score = if outcomes.is_empty() {
        0
    } else {
        (outcomes.iter().map(|o| o.score as f64).sum::<f64>() / outcomes.len() as f64).round()
            as u32
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
    for (index, question) in plan.open_questions.iter().enumerate() {
        let note = plan
            .notes
            .iter()
            .find(|note| note.topic_id == question.topic_id);
        let answer = open_answers
            .get(&question.topic_id)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("(студент не дав відповіді)");

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

    let prompt = GRADE_OPEN_PROMPT.replace("{{ANSWERS}}", &block);
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
        .filter(|grading| {
            plan.open_questions
                .iter()
                .any(|question| question.topic_id == grading.topic_id)
        })
        .collect())
}

fn session_path(vault: &Vault, subject_id: &str, session_id: &str) -> std::path::PathBuf {
    vault
        .sessions_dir()
        .join(format!("{subject_id}-{session_id}.json"))
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    plan: SessionPlan,
    #[serde(default)]
    result: Option<SessionResult>,
}

fn save_session(vault: &Vault, plan: &SessionPlan, result: Option<&SessionResult>) -> Result<()> {
    let dir = vault.sessions_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let stored = StoredSession {
        plan: plan.clone(),
        result: result.cloned(),
    };
    let path = session_path(vault, &plan.subject, &plan.id);
    std::fs::write(&path, serde_json::to_string_pretty(&stored)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn load_session(vault: &Vault, subject_id: &str, session_id: &str) -> Result<SessionPlan> {
    let path = session_path(vault, subject_id, session_id);
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading session {}", path.display()))?;
    let stored: StoredSession = serde_json::from_str(&raw)
        .with_context(|| format!("parsing session {}", path.display()))?;
    Ok(stored.plan)
}


/// A session that was started but never graded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub subject: String,
    pub created_at: String,
    pub topic_titles: Vec<String>,
}

/// Unfinished sessions for a subject, newest first, so an interrupted one can be resumed
/// instead of paying for a fresh generation call.
pub fn list_unfinished(vault: &Vault, subject_id: &str) -> Vec<SessionSummary> {
    let Ok(entries) = std::fs::read_dir(vault.sessions_dir()) else {
        return Vec::new();
    };
    let prefix = format!("{subject_id}-");

    let mut out: Vec<SessionSummary> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|raw| serde_json::from_str::<StoredSession>(&raw).ok())
        .filter(|stored| stored.result.is_none())
        .map(|stored| SessionSummary {
            id: stored.plan.id,
            subject: stored.plan.subject,
            created_at: stored.plan.created_at,
            topic_titles: stored
                .plan
                .topics
                .into_iter()
                .map(|entry| entry.topic.title)
                .collect(),
        })
        .collect();

    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}
