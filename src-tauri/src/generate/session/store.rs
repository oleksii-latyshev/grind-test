//! Sessions on disk: saved when planned, so quitting mid-session loses nothing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::grading::SessionResult;
use super::plan::SessionPlan;
use crate::vault::Vault;

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

pub(super) fn save_session(
    vault: &Vault,
    plan: &SessionPlan,
    result: Option<&SessionResult>,
) -> Result<()> {
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
