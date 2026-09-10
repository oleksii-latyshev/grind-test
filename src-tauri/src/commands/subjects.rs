//! The subject list, one subject in detail, and generating its knowledge notes.

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

use super::{fail, AppState, CmdResult, GENERATION_EVENT};
use crate::generate::{self, GenerationEvent, KnowledgeBatchReport};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::progress::{self, SubjectStats};
use crate::vault::quiz::{self, QuizSummary};
use crate::vault::study::{self, StudyOverview, TopicStudy};
use crate::vault::syllabus::{self, Subject, Topic};
use crate::vault::Vault;

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
    pub study: StudyOverview,
    /// Per-topic study state, keyed by topic id. Absent means never studied.
    pub topic_study: BTreeMap<String, TopicStudy>,
}

fn knowledge_ids(vault: &Vault, topics: &[&Topic]) -> Vec<String> {
    topics
        .iter()
        .filter(|topic| knowledge::exists(vault, topic))
        .map(|topic| topic.id.clone())
        .collect()
}

#[tauri::command]
pub fn list_subjects(state: State<'_, AppState>) -> CmdResult<Vec<SubjectOverview>> {
    let vault = state.vault();
    let vault = &vault;
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
    let vault = state.vault();
    let vault = &vault;
    let subject = syllabus::load(vault, &subject_id).map_err(fail)?;
    let topics: Vec<&Topic> = subject.topics().collect();
    let ids = knowledge_ids(vault, &topics);
    let mastery = progress::load_mastery(vault);
    let stats = progress::subject_stats(&mastery, &subject_id, &topics, ids.len());
    let quizzes = quiz::list(vault, &subject_id).map_err(fail)?;

    // Only topics with a note can be studied, so the overview is scoped to those.
    let state = study::load(vault);
    let studiable: Vec<&Topic> = topics
        .iter()
        .copied()
        .filter(|topic| ids.contains(&topic.id))
        .collect();
    let overview = study::overview(&state, &subject_id, &studiable);
    let topic_study = topics
        .iter()
        .filter_map(|topic| {
            state
                .topics
                .get(&topic.id)
                .map(|entry| (topic.id.clone(), entry.clone()))
        })
        .collect();

    Ok(SubjectDetail {
        knowledge_topic_ids: ids,
        quizzes,
        stats,
        study: overview,
        topic_study,
        subject,
    })
}

#[tauri::command]
pub fn get_knowledge(
    state: State<'_, AppState>,
    subject_id: String,
    topic_id: String,
) -> CmdResult<Option<KnowledgeNote>> {
    let vault = state.vault();
    let vault = &vault;
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
    let vault = app.state::<AppState>().vault();
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
