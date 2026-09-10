//! Study sessions: plan the topics from disk, generate the questions while the notes are read,
//! grade every written answer in one call, and keep the whole session on disk.

mod grading;
mod mode;
mod plan;
mod store;

pub use grading::{finish_session, SessionResult};
pub use mode::{Selection, SessionMode};
pub use plan::{plan_session, prepare_questions, start_session, SessionPlan};
pub use store::{list_unfinished, load_session, SessionSummary};
