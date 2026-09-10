//! Study mode: what to learn next, and when to come back to it.
//!
//! A session walks a handful of topics through read → open answers → quiz, then schedules
//! each topic for review on a Leitner-style ladder.

use anyhow::{Context, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::paths::Vault;
use super::syllabus::Topic;

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

/// Choose the next topics to study: overdue reviews first, then new material in syllabus
/// order so the subject is covered front to back rather than at random.
pub fn plan_session(state: &StudyState, candidates: &[&Topic], size: usize) -> Vec<PlannedTopic> {
    let now = chrono::Utc::now();

    let mut due: Vec<(&Topic, &TopicStudy)> = candidates
        .iter()
        .filter_map(|topic| state.topics.get(&topic.id).map(|entry| (*topic, entry)))
        .filter(|(_, entry)| entry.is_due(now))
        .collect();
    due.sort_by(|(_, a), (_, b)| a.due_at.cmp(&b.due_at).then(a.level.cmp(&b.level)));

    let mut planned: Vec<PlannedTopic> = due
        .into_iter()
        .take(size)
        .map(|(topic, _)| planned_for(state, topic))
        .collect();

    for topic in candidates {
        if planned.len() >= size {
            break;
        }
        if state.topics.contains_key(&topic.id) {
            continue;
        }
        planned.push(planned_for(state, topic));
    }

    planned
}

/// Coverage-first ordering for a sprint: everything unseen, in syllabus order, before
/// anything already met is revisited.
///
/// The ladder in `plan_session` optimises for retention over weeks. A sprint has days, so
/// the priority inverts: touch every topic once, then come back to the weakest.
pub fn plan_sprint(state: &StudyState, candidates: &[&Topic], size: usize) -> Vec<PlannedTopic> {
    let mut planned: Vec<PlannedTopic> = candidates
        .iter()
        .filter(|topic| !state.topics.contains_key(&topic.id))
        .take(size)
        .map(|topic| planned_for(state, topic))
        .collect();

    if planned.len() < size {
        let mut seen: Vec<&&Topic> = candidates
            .iter()
            .filter(|topic| state.topics.contains_key(&topic.id))
            .collect();
        // Weakest first, oldest first among equals.
        seen.sort_by(|a, b| {
            let score = |topic: &Topic| {
                state
                    .topics
                    .get(&topic.id)
                    .and_then(|entry| entry.last_score)
                    .unwrap_or(0)
            };
            score(a)
                .cmp(&score(b))
                .then_with(|| {
                    let seen_at = |topic: &Topic| {
                        state
                            .topics
                            .get(&topic.id)
                            .and_then(|entry| entry.last_studied.clone())
                    };
                    seen_at(a).cmp(&seen_at(b))
                })
        });
        for topic in seen {
            if planned.len() >= size {
                break;
            }
            planned.push(planned_for(state, topic));
        }
    }

    planned
}

/// Pick topics from across the syllabus rather than consecutively.
///
/// The exam draws its questions from different parts of the course — one from the early
/// material, one from the middle, one from the end — so revision that always walks the
/// syllabus front to back never rehearses that jump. The candidates are cut into as many
/// equal slices as there are topics to pick, and one comes from each, chosen at random but
/// weighted toward what the student knows least.
pub fn plan_spread(state: &StudyState, candidates: &[&Topic], size: usize) -> Vec<PlannedTopic> {
    if size == 0 {
        return Vec::new();
    }
    if candidates.len() <= size {
        return candidates
            .iter()
            .map(|topic| planned_for(state, topic))
            .collect();
    }

    let mut rng = rand::thread_rng();
    let mut planned = Vec::with_capacity(size);
    for bucket in 0..size {
        let start = bucket * candidates.len() / size;
        let end = ((bucket + 1) * candidates.len() / size).max(start + 1);
        if let Some(topic) = weighted_pick(state, &candidates[start..end], &mut rng) {
            planned.push(planned_for(state, topic));
        }
    }
    planned
}

/// Notes-only reading: the spread, restricted to topics never opened.
///
/// The spread alone leans toward the weakest topics, and a session whose tests were skipped
/// records every topic as 0 — so without this filter a reader who never takes the tests is
/// served the same notes again and again instead of new ones.
pub fn plan_reading(state: &StudyState, candidates: &[&Topic], size: usize) -> Vec<PlannedTopic> {
    let unseen: Vec<&Topic> = candidates
        .iter()
        .copied()
        .filter(|topic| !state.topics.contains_key(&topic.id))
        .collect();
    plan_spread(state, &unseen, size)
}

/// How much a topic needs attention: unseen outranks answered-but-shakily, which outranks
/// answered well.
fn need(state: &StudyState, topic: &Topic) -> f64 {
    match state.topics.get(&topic.id) {
        None => 1.5,
        Some(entry) => 0.3 + 2.0 * (1.0 - f64::from(entry.last_score.unwrap_or(0)) / 100.0),
    }
}

fn weighted_pick<'a>(
    state: &StudyState,
    slice: &[&'a Topic],
    rng: &mut impl Rng,
) -> Option<&'a Topic> {
    let total: f64 = slice.iter().map(|topic| need(state, topic)).sum();
    if total <= 0.0 {
        return slice.first().copied();
    }
    let mut roll = rng.gen_range(0.0..total);
    for topic in slice {
        let weight = need(state, topic);
        if roll < weight {
            return Some(topic);
        }
        roll -= weight;
    }
    slice.last().copied()
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
        } else {
            due_now += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(id: &str) -> Topic {
        Topic {
            id: id.into(),
            subject: "demo".into(),
            section: 1,
            section_title: "s".into(),
            index: 1,
            title: id.into(),
            group: None,
        }
    }

    #[test]
    fn a_good_score_advances_and_a_bad_one_falls_back() {
        assert_eq!(next_level(0, 90), 1);
        assert_eq!(next_level(3, 90), 4);
        assert_eq!(next_level(3, 70), 3);
        assert_eq!(next_level(3, 40), 2);
        // Never drops back to "never studied", and never runs off the ladder.
        assert_eq!(next_level(1, 10), 1);
        assert_eq!(next_level(MAX_LEVEL, 100), MAX_LEVEL);
    }

    #[test]
    fn planning_puts_due_reviews_before_new_material() {
        let topics: Vec<Topic> = (1..=5).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();

        let mut state = StudyState::default();
        state.topics.insert(
            "demo/1.4".into(),
            TopicStudy {
                topic_id: "demo/1.4".into(),
                level: 2,
                sessions: 1,
                due_at: Some((chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339()),
                ..Default::default()
            },
        );

        let planned = plan_session(&state, &refs, 3);
        assert_eq!(planned.len(), 3);
        assert_eq!(planned[0].topic.id, "demo/1.4");
        assert!(planned[0].is_review);
        // The rest is fresh material, taken in syllabus order.
        assert_eq!(planned[1].topic.id, "demo/1.1");
        assert_eq!(planned[2].topic.id, "demo/1.2");
    }

    #[test]
    fn a_topic_not_yet_due_is_left_alone() {
        let topics = vec![topic("demo/1.1")];
        let refs: Vec<&Topic> = topics.iter().collect();
        let mut state = StudyState::default();
        state.topics.insert(
            "demo/1.1".into(),
            TopicStudy {
                topic_id: "demo/1.1".into(),
                level: 3,
                sessions: 1,
                due_at: Some((chrono::Utc::now() + chrono::Duration::days(5)).to_rfc3339()),
                ..Default::default()
            },
        );
        assert!(plan_session(&state, &refs, 3).is_empty());
    }

    #[test]
    fn a_sprint_covers_new_ground_before_repeating_anything() {
        let topics: Vec<Topic> = (1..=5).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();

        let mut state = StudyState::default();
        // Studied, and overdue — the ladder would serve it first; a sprint should not.
        state.topics.insert(
            "demo/1.1".into(),
            TopicStudy {
                topic_id: "demo/1.1".into(),
                level: 1,
                sessions: 1,
                last_score: Some(50),
                due_at: Some((chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339()),
                ..Default::default()
            },
        );

        let planned = plan_sprint(&state, &refs, 3);
        let ids: Vec<&str> = planned.iter().map(|entry| entry.topic.id.as_str()).collect();
        assert_eq!(ids, ["demo/1.2", "demo/1.3", "demo/1.4"]);
    }

    #[test]
    fn a_sprint_falls_back_to_the_weakest_once_everything_is_seen() {
        let topics: Vec<Topic> = (1..=3).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();

        let mut state = StudyState::default();
        for (id, score) in [("demo/1.1", 90), ("demo/1.2", 30), ("demo/1.3", 60)] {
            state.topics.insert(
                id.into(),
                TopicStudy {
                    topic_id: id.into(),
                    level: 1,
                    sessions: 1,
                    last_score: Some(score),
                    ..Default::default()
                },
            );
        }

        let planned = plan_sprint(&state, &refs, 2);
        let ids: Vec<&str> = planned.iter().map(|entry| entry.topic.id.as_str()).collect();
        assert_eq!(ids, ["demo/1.2", "demo/1.3"]);
    }

    #[test]
    fn a_spread_takes_one_topic_from_each_part_of_the_syllabus() {
        let topics: Vec<Topic> = (1..=12).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();
        let state = StudyState::default();

        // Buckets are [0..4), [4..8), [8..12): the beginning, the middle and the end.
        for _ in 0..50 {
            let planned = plan_spread(&state, &refs, 3);
            assert_eq!(planned.len(), 3);

            let positions: Vec<usize> = planned
                .iter()
                .map(|entry| {
                    refs.iter()
                        .position(|topic| topic.id == entry.topic.id)
                        .expect("planned topic came from the candidate list")
                })
                .collect();

            assert!((0..4).contains(&positions[0]), "{positions:?}");
            assert!((4..8).contains(&positions[1]), "{positions:?}");
            assert!((8..12).contains(&positions[2]), "{positions:?}");
        }
    }

    #[test]
    fn a_spread_prefers_the_weakest_topic_in_its_slice() {
        let topics: Vec<Topic> = (1..=4).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();

        let mut state = StudyState::default();
        // Everything in the first half is known cold except 1.2.
        for (id, score) in [("demo/1.1", 100), ("demo/1.2", 10), ("demo/1.3", 100), ("demo/1.4", 100)] {
            state.topics.insert(
                id.into(),
                TopicStudy {
                    topic_id: id.into(),
                    level: 2,
                    sessions: 1,
                    last_score: Some(score),
                    ..Default::default()
                },
            );
        }

        let picks = (0..200)
            .filter(|_| plan_spread(&state, &refs, 2)[0].topic.id == "demo/1.2")
            .count();
        // 1.2 carries weight 2.1 against 1.1's 0.3, so it should dominate its slice.
        assert!(picks > 120, "weak topic picked only {picks}/200 times");
    }

    #[test]
    fn reading_never_serves_a_topic_already_opened() {
        let topics: Vec<Topic> = (1..=9).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();

        let mut state = StudyState::default();
        // Opened with the tests skipped: exactly what the spread would otherwise favour.
        for id in ["demo/1.1", "demo/1.5", "demo/1.9"] {
            state.topics.insert(
                id.into(),
                TopicStudy {
                    topic_id: id.into(),
                    level: 1,
                    read_count: 1,
                    last_score: Some(0),
                    ..Default::default()
                },
            );
        }

        for _ in 0..50 {
            let planned = plan_reading(&state, &refs, 3);
            assert_eq!(planned.len(), 3);
            assert!(
                planned.iter().all(|entry| !state.topics.contains_key(&entry.topic.id)),
                "{:?}",
                planned.iter().map(|entry| &entry.topic.id).collect::<Vec<_>>()
            );
        }

        for topic in &topics {
            state.topics.entry(topic.id.clone()).or_default();
        }
        assert!(plan_reading(&state, &refs, 3).is_empty());
    }

    #[test]
    fn a_spread_of_everything_returns_everything() {
        let topics: Vec<Topic> = (1..=3).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();
        assert_eq!(plan_spread(&StudyState::default(), &refs, 5).len(), 3);
    }

    #[test]
    fn overview_counts_stages_and_progress() {
        let topics: Vec<Topic> = (1..=4).map(|i| topic(&format!("demo/1.{i}"))).collect();
        let refs: Vec<&Topic> = topics.iter().collect();
        let mut state = StudyState::default();
        state.topics.insert(
            "demo/1.1".into(),
            TopicStudy {
                topic_id: "demo/1.1".into(),
                level: MAX_LEVEL,
                ..Default::default()
            },
        );
        state.topics.insert(
            "demo/1.2".into(),
            TopicStudy {
                topic_id: "demo/1.2".into(),
                level: 1,
                ..Default::default()
            },
        );

        let view = overview(&state, "demo", &refs);
        assert_eq!(view.available, 4);
        assert_eq!(view.new, 2);
        assert_eq!(view.mastered, 1);
        assert_eq!(view.learning, 1);
        // (6 + 1) out of a ceiling of 4 * 6.
        assert_eq!(view.progress_percent, 29);
    }
}
