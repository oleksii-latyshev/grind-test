//! Attempt history and the mastery table it feeds — the "memory" that later quizzes
//! are weighted against.

use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::paths::Vault;
use super::quiz::Quiz;
use super::syllabus::Topic;

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

/// How badly a topic needs revisiting. Never zero — a mastered topic should still come
/// round occasionally.
pub fn weight(mastery: &Mastery, topic_id: &str) -> f64 {
    match mastery.topics.get(topic_id) {
        // Unseen topics rank just above a topic answered correctly every time, so a fresh
        // vault sweeps for breadth before it starts drilling.
        None => 1.2,
        Some(entry) => 0.2 + 2.0 * (1.0 - entry.accuracy()),
    }
}

/// Weighted sampling without replacement, biased toward topics the user keeps missing.
pub fn pick_topics<'a>(
    mastery: &Mastery,
    candidates: &[&'a Topic],
    count: usize,
) -> Vec<&'a Topic> {
    if candidates.len() <= count {
        return candidates.to_vec();
    }

    let mut pool: Vec<(&Topic, f64)> = candidates
        .iter()
        .map(|topic| (*topic, weight(mastery, &topic.id)))
        .collect();
    let mut rng = rand::thread_rng();
    pool.shuffle(&mut rng);

    let mut picked = Vec::with_capacity(count);
    for _ in 0..count {
        let total: f64 = pool.iter().map(|(_, w)| w).sum();
        if total <= 0.0 || pool.is_empty() {
            break;
        }
        let mut roll = rng.gen_range(0.0..total);
        let mut chosen = pool.len() - 1;
        for (index, (_, w)) in pool.iter().enumerate() {
            if roll < *w {
                chosen = index;
                break;
            }
            roll -= *w;
        }
        picked.push(pool.remove(chosen).0);
    }
    picked
}

/// Dashboard numbers for one subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectStats {
    pub subject: String,
    pub topics_total: usize,
    pub topics_with_knowledge: usize,
    pub topics_practised: usize,
    pub questions_answered: u32,
    pub questions_correct: u32,
    pub accuracy_percent: u32,
    pub weakest: Vec<WeakTopic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakTopic {
    pub topic_id: String,
    pub title: String,
    pub seen: u32,
    pub correct: u32,
    pub accuracy_percent: u32,
}

pub fn subject_stats(
    mastery: &Mastery,
    subject: &str,
    topics: &[&Topic],
    topics_with_knowledge: usize,
) -> SubjectStats {
    let mut answered = 0;
    let mut correct = 0;
    let mut practised = 0;
    let mut weakest = Vec::new();

    for topic in topics {
        let Some(entry) = mastery.topics.get(&topic.id) else {
            continue;
        };
        if entry.seen == 0 {
            continue;
        }
        practised += 1;
        answered += entry.seen;
        correct += entry.correct;
        weakest.push(WeakTopic {
            topic_id: topic.id.clone(),
            title: topic.title.clone(),
            seen: entry.seen,
            correct: entry.correct,
            accuracy_percent: (entry.accuracy() * 100.0).round() as u32,
        });
    }

    weakest.sort_by(|a, b| {
        a.accuracy_percent
            .cmp(&b.accuracy_percent)
            .then(b.seen.cmp(&a.seen))
    });
    weakest.truncate(10);

    SubjectStats {
        subject: subject.to_string(),
        topics_total: topics.len(),
        topics_with_knowledge,
        topics_practised: practised,
        questions_answered: answered,
        questions_correct: correct,
        accuracy_percent: if answered == 0 {
            0
        } else {
            ((correct as f64 / answered as f64) * 100.0).round() as u32
        },
        weakest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(id: &str) -> Topic {
        Topic {
            id: id.into(),
            subject: "f3".into(),
            section: 1,
            section_title: "s".into(),
            index: 1,
            title: id.into(),
            group: None,
        }
    }

    #[test]
    fn missed_topics_outweigh_mastered_ones() {
        let mut mastery = Mastery::default();
        mastery.topics.insert(
            "good".into(),
            TopicMastery {
                topic_id: "good".into(),
                seen: 10,
                correct: 10,
                ..Default::default()
            },
        );
        mastery.topics.insert(
            "bad".into(),
            TopicMastery {
                topic_id: "bad".into(),
                seen: 10,
                correct: 1,
                ..Default::default()
            },
        );
        assert!(weight(&mastery, "bad") > weight(&mastery, "unseen"));
        assert!(weight(&mastery, "unseen") > weight(&mastery, "good"));
    }

    #[test]
    fn picking_returns_the_requested_count_without_repeats() {
        let topics: Vec<Topic> = (0..10).map(|i| topic(&format!("t{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();
        let picked = pick_topics(&Mastery::default(), &refs, 4);
        assert_eq!(picked.len(), 4);
        let mut ids: Vec<&str> = picked.iter().map(|t| t.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn picking_more_than_available_returns_everything() {
        let topics: Vec<Topic> = (0..3).map(|i| topic(&format!("t{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();
        assert_eq!(pick_topics(&Mastery::default(), &refs, 10).len(), 3);
    }
}
