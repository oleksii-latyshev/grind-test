//! Attempt history and the mastery table it feeds — the "memory" that later quizzes
//! are weighted against.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::paths::Vault;
use super::quiz::Quiz;

mod stats;
mod weighting;

pub use stats::{subject_stats, SubjectStats, WeakTopic};
pub use weighting::{pick_topics, weight};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerRecord {
    pub question_id: String,
    pub topic_id: String,
    pub selected: Vec<usize>,
    pub correct: bool,
    #[serde(default)]
    pub hint_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub quiz_id: String,
    pub subject: String,
    pub quiz_title: String,
    pub finished_at: String,
    pub answers: Vec<AnswerRecord>,
}

impl Attempt {
    pub fn correct_count(&self) -> usize {
        self.answers.iter().filter(|a| a.correct).count()
    }

    pub fn score_percent(&self) -> u32 {
        if self.answers.is_empty() {
            return 0;
        }
        ((self.correct_count() as f64 / self.answers.len() as f64) * 100.0).round() as u32
    }
}

/// What the app hands back after grading, so the review screen needs no second call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptResult {
    pub attempt: Attempt,
    pub total: usize,
    pub correct: usize,
    pub score_percent: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicMastery {
    pub topic_id: String,
    pub seen: u32,
    pub correct: u32,
    #[serde(default)]
    pub hints_used: u32,
    pub last_seen: String,
}

impl TopicMastery {
    pub fn accuracy(&self) -> f64 {
        if self.seen == 0 {
            0.0
        } else {
            self.correct as f64 / self.seen as f64
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mastery {
    #[serde(default)]
    pub topics: BTreeMap<String, TopicMastery>,
}

pub fn load_mastery(vault: &Vault) -> Mastery {
    let path = vault.mastery_file();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_mastery(vault: &Vault, mastery: &Mastery) -> Result<()> {
    let path = vault.mastery_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(mastery)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Grade the raw selections against the quiz, persist the attempt, and fold it into mastery.
pub fn record_attempt(
    vault: &Vault,
    quiz: &Quiz,
    selections: &BTreeMap<String, Vec<usize>>,
    hints_used: &[String],
) -> Result<AttemptResult> {
    let now = chrono::Utc::now().to_rfc3339();
    let answers: Vec<AnswerRecord> = quiz
        .questions
        .iter()
        .map(|question| {
            let selected = selections.get(&question.id).cloned().unwrap_or_default();
            AnswerRecord {
                question_id: question.id.clone(),
                topic_id: question.topic_id.clone(),
                correct: question.is_correct(&selected),
                selected,
                hint_used: hints_used.contains(&question.id),
            }
        })
        .collect();

    let attempt = Attempt {
        id: uuid::Uuid::new_v4().to_string(),
        quiz_id: quiz.id.clone(),
        subject: quiz.subject.clone(),
        quiz_title: quiz.title.clone(),
        finished_at: now.clone(),
        answers,
    };

    let dir = vault.attempts_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{}.json", attempt.id));
    std::fs::write(&path, serde_json::to_string_pretty(&attempt)?)
        .with_context(|| format!("writing {}", path.display()))?;

    apply_answers(vault, &attempt.answers)?;

    Ok(AttemptResult {
        total: attempt.answers.len(),
        correct: attempt.correct_count(),
        score_percent: attempt.score_percent(),
        attempt,
    })
}

/// Fold a batch of graded answers into the mastery table. Shared by full quizzes and by
/// the mini-quiz inside a study session, so both feed the same weighting.
pub fn apply_answers(vault: &Vault, answers: &[AnswerRecord]) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut mastery = load_mastery(vault);
    for answer in answers {
        let entry = mastery
            .topics
            .entry(answer.topic_id.clone())
            .or_insert_with(|| TopicMastery {
                topic_id: answer.topic_id.clone(),
                ..Default::default()
            });
        entry.seen += 1;
        if answer.correct {
            entry.correct += 1;
        }
        if answer.hint_used {
            entry.hints_used += 1;
        }
        entry.last_seen = now.clone();
    }
    save_mastery(vault, &mastery)
}

pub fn list_attempts(vault: &Vault, subject: Option<&str>) -> Vec<Attempt> {
    let dir = vault.attempts_dir();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Attempt> = entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|raw| serde_json::from_str::<Attempt>(&raw).ok())
        .filter(|attempt| subject.is_none_or(|s| attempt.subject == s))
        .collect();
    out.sort_by(|a, b| b.finished_at.cmp(&a.finished_at));
    out
}
