//! Practice quizzes: generating, taking and grading them, hints, and the attempt history.

use std::collections::BTreeMap;
use tauri::{Manager, State};

use super::{fail, AppState, CmdResult};
use crate::generate::{self, Hint, QuizRequest};
use crate::vault::progress::{self, Attempt, AttemptResult};
use crate::vault::quiz::{self, Quiz, QuizSummary};

#[tauri::command]
pub async fn generate_quiz(app: tauri::AppHandle, request: QuizRequest) -> CmdResult<Quiz> {
    let vault = app.state::<AppState>().vault();
    generate::quiz(&vault, request).await.map_err(fail)
}

#[tauri::command]
pub fn list_quizzes(state: State<'_, AppState>, subject_id: String) -> CmdResult<Vec<QuizSummary>> {
    quiz::list(&state.vault(), &subject_id).map_err(fail)
}

#[tauri::command]
pub fn get_quiz(
    state: State<'_, AppState>,
    subject_id: String,
    quiz_id: String,
) -> CmdResult<Quiz> {
    quiz::read(&state.vault(), &subject_id, &quiz_id).map_err(fail)
}

#[tauri::command]
pub fn submit_quiz(
    state: State<'_, AppState>,
    subject_id: String,
    quiz_id: String,
    selections: BTreeMap<String, Vec<usize>>,
    hints_used: Vec<String>,
) -> CmdResult<AttemptResult> {
    generate::grade(
        &state.vault(),
        &subject_id,
        &quiz_id,
        selections,
        hints_used,
    )
    .map_err(fail)
}

#[tauri::command]
pub async fn get_hint(
    app: tauri::AppHandle,
    subject_id: String,
    quiz_id: String,
    question_id: String,
) -> CmdResult<Hint> {
    let vault = app.state::<AppState>().vault();
    generate::hint(&vault, &subject_id, &quiz_id, &question_id)
        .await
        .map_err(fail)
}

#[tauri::command]
pub fn list_attempts(state: State<'_, AppState>, subject_id: Option<String>) -> Vec<Attempt> {
    progress::list_attempts(&state.vault(), subject_id.as_deref())
}
