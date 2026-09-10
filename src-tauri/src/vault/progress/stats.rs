//! Dashboard numbers for one subject.

use serde::{Deserialize, Serialize};

use super::Mastery;
use crate::vault::syllabus::Topic;

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
