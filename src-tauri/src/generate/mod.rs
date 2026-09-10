//! Prompt assembly and the generation jobs: knowledge notes, quizzes, hints and study sessions.
//!
//! Prompts and schemas are embedded at compile time so a packaged app never depends on
//! files sitting next to the binary.

mod batch;
mod hint;
mod knowledge;
mod questions;
mod quiz;
mod session;

use std::time::Duration;

pub use batch::{knowledge_batch, EventSink, GenerationEvent, KnowledgeBatchReport};
pub use hint::{hint, Hint};
pub use quiz::{grade, quiz, QuizRequest};
pub use session::{
    finish_session, list_unfinished, load_session, plan_session, prepare_questions, start_session,
    Selection, SessionMode, SessionPlan, SessionResult, SessionSummary,
};

const KNOWLEDGE_TIMEOUT: Duration = Duration::from_secs(420);
const QUIZ_TIMEOUT: Duration = Duration::from_secs(420);

fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_string(),
        Some((cut, _)) => format!("{}\n…", &text[..cut]),
    }
}
