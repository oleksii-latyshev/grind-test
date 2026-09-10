//! Study mode: what to learn next, and when to come back to it.
//!
//! A session walks a handful of topics through read → open answers → quiz, then schedules
//! each topic for review on a Leitner-style ladder.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::paths::Vault;
use super::syllabus::Topic;

mod planning;
#[cfg(test)]
mod tests;

pub use planning::{plan_reading, plan_session, plan_spread, plan_sprint};

/// Days until review for a topic sitting at level 1..=6. Tight at the start because the
/// exam is close; the tail exists so mastered material still resurfaces.
const REVIEW_INTERVALS_DAYS: [i64; 6] = [1, 2, 4, 7, 14, 30];

/// At or above this a topic advances a level.
const PASS_SCORE: u32 = 80;
/// Below this it drops one.
const FAIL_SCORE: u32 = 60;

pub const MAX_LEVEL: u8 = 6;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicStudy {
    pub topic_id: String,
    /// 0 = never studied. 1..=6 index into the review ladder.
    pub level: u8,
    pub read_count: u32,
    pub sessions: u32,
    pub last_studied: Option<String>,
    pub due_at: Option<String>,
    pub last_score: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Never opened.
    New,
    /// Read but not yet tested.
    Reading,
    /// Being drilled.
    Learning,
    /// Held up under review.
    Review,
    Mastered,
}

impl TopicStudy {
    pub fn stage(&self) -> Stage {
        match self.level {
            0 if self.read_count == 0 => Stage::New,
            0 => Stage::Reading,
            1..=2 => Stage::Learning,
            3..=4 => Stage::Review,
            _ => Stage::Mastered,
        }
    }

    pub fn is_due(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        match &self.due_at {
            // Read but never tested — always worth finishing.
            None => self.level == 0,
            Some(due) => chrono::DateTime::parse_from_rfc3339(due)
                .map(|due| due <= now)
                .unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StudyState {
    #[serde(default)]
    pub topics: BTreeMap<String, TopicStudy>,
}

impl StudyState {
    fn entry(&mut self, topic_id: &str) -> &mut TopicStudy {
        self.topics
            .entry(topic_id.to_string())
            .or_insert_with(|| TopicStudy {
                topic_id: topic_id.to_string(),
                ..Default::default()
            })
    }
}

pub fn load(vault: &Vault) -> StudyState {
    std::fs::read_to_string(vault.study_file())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(vault: &Vault, state: &StudyState) -> Result<()> {
    let path = vault.study_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(state)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn mark_read(vault: &Vault, topic_id: &str) -> Result<()> {
    let mut state = load(vault);
    let entry = state.entry(topic_id);
    entry.read_count += 1;
    entry.last_studied = Some(chrono::Utc::now().to_rfc3339());
    save(vault, &state)
}

/// Apply one session's per-topic scores (0..=100) and reschedule each topic.
pub fn record_scores(vault: &Vault, scores: &BTreeMap<String, u32>) -> Result<StudyState> {
    let now = chrono::Utc::now();
    let mut state = load(vault);

    for (topic_id, score) in scores {
        let entry = state.entry(topic_id);
        entry.sessions += 1;
        entry.last_score = Some(*score);
        entry.last_studied = Some(now.to_rfc3339());
        entry.level = next_level(entry.level, *score);
        let days = REVIEW_INTERVALS_DAYS[(entry.level.max(1) - 1) as usize];
        entry.due_at = Some((now + chrono::Duration::days(days)).to_rfc3339());
    }

    save(vault, &state)?;
    Ok(state)
}

fn next_level(level: u8, score: u32) -> u8 {
    if score >= PASS_SCORE {
        (level + 1).min(MAX_LEVEL)
    } else if score < FAIL_SCORE {
        level.saturating_sub(1).max(1)
    } else {
        level.max(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTopic {
    pub topic: Topic,
    pub stage: Stage,
    pub level: u8,
    pub last_score: Option<u32>,
    /// True when this is a scheduled review rather than fresh material.
    pub is_review: bool,
}

/// Describe a topic's current standing, whether or not it has ever been studied.
pub fn planned_for(state: &StudyState, topic: &Topic) -> PlannedTopic {
    match state.topics.get(&topic.id) {
        Some(entry) => PlannedTopic {
            topic: topic.clone(),
            stage: entry.stage(),
            level: entry.level,
            last_score: entry.last_score,
            is_review: entry.level > 0,
        },
        None => PlannedTopic {
            topic: topic.clone(),
            stage: Stage::New,
            level: 0,
            last_score: None,
            is_review: false,
        },
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StudyOverview {
    pub subject: String,
    /// Topics that have a knowledge note and can therefore be studied.
    pub available: usize,
    pub new: usize,
    pub learning: usize,
    pub review: usize,
    pub mastered: usize,
    /// Started topics waiting for their review now. Never-studied ones are counted in `new`
    /// only — adding them here too made "due" look larger than the subject itself.
    pub due_now: usize,
    pub studied_today: usize,
    /// Weighted 0..100 across the whole subject: how much of it is actually learnt.
    pub progress_percent: u32,
}

pub fn overview(state: &StudyState, subject: &str, candidates: &[&Topic]) -> StudyOverview {
    let now = chrono::Utc::now();
    let today = now.date_naive();

    let mut counts = [0usize; 5];
    let mut due_now = 0;
    let mut studied_today = 0;
    let mut level_sum = 0u32;

    for topic in candidates {
        let entry = state.topics.get(&topic.id);
        let stage = entry.map(|e| e.stage()).unwrap_or(Stage::New);
        counts[match stage {
            Stage::New => 0,
            Stage::Reading => 1,
            Stage::Learning => 2,
            Stage::Review => 3,
            Stage::Mastered => 4,
        }] += 1;

        if let Some(entry) = entry {
            if entry.is_due(now) {
                due_now += 1;
            }
            level_sum += u32::from(entry.level);
            if entry
                .last_studied
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|value| value.date_naive() == today)
            {
                studied_today += 1;
            }
        }
    }

    let ceiling = candidates.len() as u32 * u32::from(MAX_LEVEL);
    StudyOverview {
        subject: subject.to_string(),
        available: candidates.len(),
        // `Reading` folds into `learning` for display: both mean "started, not tested".
        new: counts[0],
        learning: counts[1] + counts[2],
        review: counts[3],
        mastered: counts[4],
        due_now,
        studied_today,
        progress_percent: if ceiling == 0 {
            0
        } else {
            ((level_sum as f64 / ceiling as f64) * 100.0).round() as u32
        },
    }
}
