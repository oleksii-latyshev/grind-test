//! How strongly a quiz leans toward each topic.

use rand::seq::SliceRandom;
use rand::Rng;

use super::Mastery;
use crate::vault::syllabus::Topic;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::progress::TopicMastery;

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
