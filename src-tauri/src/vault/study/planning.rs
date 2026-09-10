//! Which topics a session serves next.

use rand::Rng;

use super::{planned_for, PlannedTopic, StudyState, TopicStudy};
use crate::vault::syllabus::Topic;

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
            score(a).cmp(&score(b)).then_with(|| {
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
