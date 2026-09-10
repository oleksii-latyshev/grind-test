//! Planning a session from disk, then generating its questions while the notes are read.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::mode::{Selection, SessionMode};
use super::store::{load_session, save_session};
use crate::agy::{self, ModelTier};
use crate::generate::questions::{sanitize_questions, QuestionDraft};
use crate::generate::{truncate_chars, QUIZ_TIMEOUT};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::quiz::Question;
use crate::vault::syllabus::{self, Topic};
use crate::vault::{study, Vault};

const SESSION_PROMPT: &str = include_str!("../../../prompts/session.md");
const SESSION_SCHEMA: &str = include_str!("../../../schemas/session.json");

/// Notes are quoted in full for a session: the questions must not test material the
/// student was never shown.
const SESSION_EXCERPT_CHARS: usize = 9000;

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
    #[serde(default)]
    pub mode: SessionMode,
    pub topics: Vec<study::PlannedTopic>,
    pub notes: Vec<KnowledgeNote>,
    /// What the student actually reads, aligned with `topics`: the full note, or its digest
    /// in a sprint.
    #[serde(default)]
    pub reading: Vec<String>,
    pub open_questions: Vec<OpenQuestion>,
    pub quiz: Vec<Question>,
}

/// Pick the topics for a session and load what the student will read. Disk only.
///
/// Deliberately separated from the generation call below: the notes are already on disk, so
/// there is no reason to make anyone watch a spinner for material that is sitting there. The
/// reader opens on this, and the model call runs while the first note is being read.
///
/// `topic_ids` overrides the scheduler — the student can insist on the topics they know are
/// coming up, rather than the ones the ladder would serve next.
pub fn plan_session(
    vault: &Vault,
    subject_id: &str,
    size: usize,
    topic_ids: Option<Vec<String>>,
    mode: SessionMode,
    selection: Selection,
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
                .take(mode.max_topics())
                .map(|topic| study::planned_for(&state, topic))
                .collect();
            if chosen.is_empty() {
                bail!("none of the chosen topics has a knowledge note yet");
            }
            chosen
        }
        _ => {
            let size = size.clamp(1, mode.max_topics());
            match (selection, mode) {
                (Selection::Spread, _) => study::plan_spread(&state, &studiable, size),
                (Selection::Scheduled, SessionMode::Sprint) => {
                    study::plan_sprint(&state, &studiable, size)
                }
                (Selection::Scheduled, _) => study::plan_session(&state, &studiable, size),
            }
        }
    };
    if planned.is_empty() {
        bail!("nothing is due right now — every topic in this subject is scheduled for later");
    }

    let mut notes = Vec::new();
    let mut reading = Vec::new();
    let mut topics = Vec::new();
    for entry in planned {
        let Some(note) = knowledge::read(vault, &entry.topic)? else {
            continue;
        };
        // What the student is shown is decided here and stored, because the questions are
        // generated from exactly this text — a sprint can never be tested on material its
        // digest left out.
        reading.push(if mode.reads_digest() {
            knowledge::digest(&note.body)
        } else {
            note.body.clone()
        });
        notes.push(note);
        topics.push(entry);
    }
    if notes.is_empty() {
        bail!("could not load the knowledge notes for the planned topics");
    }

    let now = chrono::Utc::now();
    let plan = SessionPlan {
        id: format!(
            "{}-{}",
            now.format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        ),
        subject: subject_id.to_string(),
        created_at: now.to_rfc3339(),
        mode,
        topics,
        notes,
        reading,
        // Filled in by `prepare_questions`; empty is what marks a session as not yet ready.
        open_questions: Vec::new(),
        quiz: Vec::new(),
    };
    save_session(vault, &plan, None)?;
    Ok(plan)
}

static PREPARE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Generate the open questions and the mini-quiz for a planned session — the one fast call
/// a session costs up front.
///
/// Idempotent: a session that already has its questions is returned untouched, so the client
/// can fire this on every mount, including a resume, without paying for it twice.
pub async fn prepare_questions(
    vault: &Vault,
    subject_id: &str,
    session_id: &str,
) -> Result<SessionPlan> {
    let plan = load_session(vault, subject_id, session_id)?;
    if !plan.quiz.is_empty() {
        return Ok(plan);
    }

    // Returning early is not enough on its own: two callers can both find an unprepared
    // session and both pay for a generation. React's StrictMode fires effects twice in
    // development, which is exactly that race. A student prepares one session at a time, so
    // one global lock is enough — and the state is re-read after acquiring it, because the
    // holder may have just finished the very session being waited on.
    let _guard = PREPARE_LOCK.lock().await;
    let mut plan = load_session(vault, subject_id, session_id)?;
    if !plan.quiz.is_empty() {
        return Ok(plan);
    }

    let mut knowledge_block = String::new();
    for (index, note) in plan.notes.iter().enumerate() {
        // `reading` is written alongside `notes`; the fallback covers a session stored
        // before that field existed.
        let shown = plan.reading.get(index).unwrap_or(&note.body);
        knowledge_block.push_str(&format!(
            "### topic_id: {}\nТема: {}\n\n{}\n\n",
            note.topic_id,
            note.title,
            truncate_chars(shown, SESSION_EXCERPT_CHARS)
        ));
    }

    let quiz_count = (plan.notes.len() * plan.mode.questions_per_topic()).clamp(6, 32);
    let prompt = SESSION_PROMPT
        .replace("{{NOTES_KIND}}", plan.mode.notes_kind())
        .replace("{{KNOWLEDGE}}", &knowledge_block)
        .replace("{{OPEN_STYLE}}", plan.mode.open_style())
        .replace("{{QUIZ_STYLE}}", plan.mode.quiz_style())
        .replace("{{QUIZ_COUNT}}", &quiz_count.to_string());

    let response =
        agy::run::<SessionDraft>(&prompt, ModelTier::Fast, SESSION_SCHEMA, QUIZ_TIMEOUT).await?;

    let topic_ids: Vec<String> = plan
        .notes
        .iter()
        .map(|note| note.topic_id.clone())
        .collect();
    let quiz = sanitize_questions(response.data.quiz_questions, &topic_ids);
    if quiz.is_empty() {
        bail!("the generator returned no usable quiz questions for this session");
    }

    // One open question per topic, in the order the student read them; anything the model
    // invented for an unknown topic is dropped.
    plan.open_questions = topic_ids
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
    plan.quiz = quiz;

    save_session(vault, &plan, None)?;
    Ok(plan)
}

/// Plan and prepare in one go. The GUI drives the two halves separately so the reading can
/// start immediately; the terminal CLI has nobody to show a note to and just waits.
pub async fn start_session(
    vault: &Vault,
    subject_id: &str,
    size: usize,
    topic_ids: Option<Vec<String>>,
    mode: SessionMode,
    selection: Selection,
) -> Result<SessionPlan> {
    let plan = plan_session(vault, subject_id, size, topic_ids, mode, selection)?;
    prepare_questions(vault, subject_id, &plan.id).await
}
