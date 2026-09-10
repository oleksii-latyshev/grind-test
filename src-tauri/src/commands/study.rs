//! Study mode: planning, preparing, resuming and grading sessions, and notes-only reading.

use std::collections::BTreeMap;
use tauri::{Manager, State};

use super::{fail, AppState, CmdResult};
use crate::generate::{self, Selection, SessionMode, SessionPlan, SessionResult};
use crate::vault::syllabus::{self, Topic};
use crate::vault::{knowledge, study};

/// Pick the topics and load their notes. Returns without calling a model, so the reader can
/// open on the first note while `prepare_session_questions` runs behind it.
///
/// The returned plan has no questions yet — an empty `quiz` is what marks a session as not
/// yet ready.
#[tauri::command]
pub fn plan_study_session(
    state: State<'_, AppState>,
    subject_id: String,
    size: Option<usize>,
    topic_ids: Option<Vec<String>>,
    mode: Option<SessionMode>,
    selection: Option<Selection>,
) -> CmdResult<SessionPlan> {
    generate::plan_session(
        &state.vault(),
        &subject_id,
        size.unwrap_or(3),
        topic_ids,
        mode.unwrap_or_default(),
        selection.unwrap_or_default(),
    )
    .map_err(fail)
}

/// The one fast-model call a session costs. Idempotent, so the client may fire it on every
/// mount — including when resuming a session that was abandoned before it finished.
#[tauri::command]
pub async fn prepare_session_questions(
    app: tauri::AppHandle,
    subject_id: String,
    session_id: String,
) -> CmdResult<SessionPlan> {
    let vault = app.state::<AppState>().vault();
    generate::prepare_questions(&vault, &subject_id, &session_id)
        .await
        .map_err(fail)
}

/// Sessions that were started but never graded, newest first.
#[tauri::command]
pub fn unfinished_sessions(
    state: State<'_, AppState>,
    subject_id: String,
) -> Vec<generate::SessionSummary> {
    generate::list_unfinished(&state.vault(), &subject_id)
}

/// Reopen a session exactly as it was generated, rather than paying for a new one.
#[tauri::command]
pub fn resume_study_session(
    state: State<'_, AppState>,
    subject_id: String,
    session_id: String,
) -> CmdResult<SessionPlan> {
    generate::load_session(&state.vault(), &subject_id, &session_id).map_err(fail)
}

#[tauri::command]
pub async fn finish_study_session(
    app: tauri::AppHandle,
    subject_id: String,
    session_id: String,
    open_answers: BTreeMap<String, String>,
    quiz_selections: BTreeMap<String, Vec<usize>>,
) -> CmdResult<SessionResult> {
    let vault = app.state::<AppState>().vault();
    generate::finish_session(
        &vault,
        &session_id,
        &subject_id,
        open_answers,
        quiz_selections,
    )
    .await
    .map_err(fail)
}

/// Notes-only reading: topics never opened, one from each part of the syllabus. Disk only,
/// and nothing is persisted — a topic leaves the pool once it is marked read.
#[tauri::command]
pub fn plan_reading(
    state: State<'_, AppState>,
    subject_id: String,
    size: usize,
) -> CmdResult<Vec<Topic>> {
    let vault = state.vault();
    let subject = syllabus::load(&vault, &subject_id).map_err(fail)?;
    let studiable: Vec<&Topic> = subject
        .topics()
        .filter(|topic| knowledge::exists(&vault, topic))
        .collect();
    Ok(study::plan_reading(&study::load(&vault), &studiable, size)
        .into_iter()
        .map(|planned| planned.topic)
        .collect())
}

/// Called when the student finishes reading a note, so a topic they have opened but not yet
/// been tested on is visibly distinct from one they have never touched.
#[tauri::command]
pub fn mark_topic_read(state: State<'_, AppState>, topic_id: String) -> CmdResult<()> {
    study::mark_read(&state.vault(), &topic_id).map_err(fail)
}
