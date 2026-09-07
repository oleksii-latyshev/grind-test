use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::generate::{self, GenerationEvent, Hint, KnowledgeBatchReport, QuizRequest};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::progress::{self, Attempt, AttemptResult, SubjectStats};
use crate::vault::quiz::{self, Quiz, QuizSummary};
use crate::vault::syllabus::{self, Subject, Topic};
use crate::vault::Vault;

/// Progress events for a running knowledge batch.
pub const GENERATION_EVENT: &str = "generation://progress";

pub struct AppState {
    pub vault: Vault,
}

type CmdResult<T> = Result<T, String>;

fn fail(error: anyhow::Error) -> String {
    format!("{error:#}")
}

#[derive(Debug, Serialize)]
pub struct SubjectOverview {
    pub id: String,
    pub title: String,
    pub topic_count: usize,
    pub knowledge_count: usize,
    pub quiz_count: usize,
    pub accuracy_percent: u32,
}

#[derive(Debug, Serialize)]
pub struct SubjectDetail {
    pub subject: Subject,
    /// Topics that already have a generated knowledge note.
    pub knowledge_topic_ids: Vec<String>,
    pub quizzes: Vec<QuizSummary>,
    pub stats: SubjectStats,
}

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub root: String,
    pub exists: bool,
    pub agy_binary: Option<String>,
}

fn knowledge_ids(vault: &Vault, topics: &[&Topic]) -> Vec<String> {
    topics
        .iter()
        .filter(|topic| knowledge::exists(vault, topic))
        .map(|topic| topic.id.clone())
        .collect()
}

#[tauri::command]
pub fn vault_info(state: State<'_, AppState>) -> VaultInfo {
    VaultInfo {
        root: state.vault.root.to_string_lossy().to_string(),
        exists: state.vault.syllabus_dir().is_dir(),
        agy_binary: crate::agy::resolve_binary()
            .ok()
            .map(|p| p.to_string_lossy().to_string()),
    }
}

#[tauri::command]
pub fn list_subjects(state: State<'_, AppState>) -> CmdResult<Vec<SubjectOverview>> {
    let vault = &state.vault;
    let subjects = syllabus::load_all(vault).map_err(fail)?;
    let mastery = progress::load_mastery(vault);

    Ok(subjects
        .into_iter()
        .map(|subject| {
            let topics: Vec<&Topic> = subject.topics().collect();
            let knowledge_count = knowledge_ids(vault, &topics).len();
            let stats = progress::subject_stats(&mastery, &subject.id, &topics, knowledge_count);
            SubjectOverview {
                id: subject.id.clone(),
                title: subject.title.clone(),
                topic_count: topics.len(),
                knowledge_count,
                quiz_count: quiz::list(vault, &subject.id).map(|q| q.len()).unwrap_or(0),
                accuracy_percent: stats.accuracy_percent,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_subject(state: State<'_, AppState>, subject_id: String) -> CmdResult<SubjectDetail> {
    let vault = &state.vault;
    let subject = syllabus::load(vault, &subject_id).map_err(fail)?;
    let topics: Vec<&Topic> = subject.topics().collect();
    let ids = knowledge_ids(vault, &topics);
    let mastery = progress::load_mastery(vault);
    let stats = progress::subject_stats(&mastery, &subject_id, &topics, ids.len());
    let quizzes = quiz::list(vault, &subject_id).map_err(fail)?;

    Ok(SubjectDetail {
        knowledge_topic_ids: ids,
        quizzes,
        stats,
        subject,
    })
}

#[tauri::command]
pub fn get_knowledge(
    state: State<'_, AppState>,
    subject_id: String,
    topic_id: String,
) -> CmdResult<Option<KnowledgeNote>> {
    let vault = &state.vault;
    let subject = syllabus::load(vault, &subject_id).map_err(fail)?;
    let Some(topic) = subject.find_topic(&topic_id) else {
        return Err(format!("unknown topic '{topic_id}'"));
    };
    knowledge::read(vault, topic).map_err(fail)
}

#[tauri::command]
pub async fn generate_knowledge(
    app: tauri::AppHandle,
    subject_id: String,
    topic_ids: Option<Vec<String>>,
    force: Option<bool>,
    concurrency: Option<usize>,
) -> CmdResult<KnowledgeBatchReport> {
    let vault = app.state::<AppState>().vault.clone();
    let emitter = app.clone();
    let sink: generate::EventSink = Arc::new(move |event: GenerationEvent| {
        let _ = emitter.emit(GENERATION_EVENT, event);
    });

    generate::knowledge_batch(
        &vault,
        &subject_id,
        topic_ids,
        force.unwrap_or(false),
        concurrency.unwrap_or(4),
        sink,
    )
    .await
    .map_err(fail)
}

#[tauri::command]
pub async fn generate_quiz(app: tauri::AppHandle, request: QuizRequest) -> CmdResult<Quiz> {
    let vault = app.state::<AppState>().vault.clone();
    generate::quiz(&vault, request).await.map_err(fail)
}

#[tauri::command]
pub fn list_quizzes(state: State<'_, AppState>, subject_id: String) -> CmdResult<Vec<QuizSummary>> {
    quiz::list(&state.vault, &subject_id).map_err(fail)
}

#[tauri::command]
pub fn get_quiz(state: State<'_, AppState>, subject_id: String, quiz_id: String) -> CmdResult<Quiz> {
    quiz::read(&state.vault, &subject_id, &quiz_id).map_err(fail)
}

#[tauri::command]
pub fn submit_quiz(
    state: State<'_, AppState>,
    subject_id: String,
    quiz_id: String,
    selections: BTreeMap<String, Vec<usize>>,
    hints_used: Vec<String>,
) -> CmdResult<AttemptResult> {
    generate::grade(&state.vault, &subject_id, &quiz_id, selections, hints_used).map_err(fail)
}

#[tauri::command]
pub async fn get_hint(
    app: tauri::AppHandle,
    subject_id: String,
    quiz_id: String,
    question_id: String,
) -> CmdResult<Hint> {
    let vault = app.state::<AppState>().vault.clone();
    generate::hint(&vault, &subject_id, &quiz_id, &question_id)
        .await
        .map_err(fail)
}

#[tauri::command]
pub fn list_attempts(state: State<'_, AppState>, subject_id: Option<String>) -> Vec<Attempt> {
    progress::list_attempts(&state.vault, subject_id.as_deref())
}
