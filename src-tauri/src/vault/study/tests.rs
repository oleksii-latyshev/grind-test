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
    let ids: Vec<&str> = planned
        .iter()
        .map(|entry| entry.topic.id.as_str())
        .collect();
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
    let ids: Vec<&str> = planned
        .iter()
        .map(|entry| entry.topic.id.as_str())
        .collect();
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
    for (id, score) in [
        ("demo/1.1", 100),
        ("demo/1.2", 10),
        ("demo/1.3", 100),
        ("demo/1.4", 100),
    ] {
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
            planned
                .iter()
                .all(|entry| !state.topics.contains_key(&entry.topic.id)),
            "{:?}",
            planned
                .iter()
                .map(|entry| &entry.topic.id)
                .collect::<Vec<_>>()
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
    // Neither studied topic has a review date, and the two unseen ones are `new`, not due.
    assert_eq!(view.due_now, 0);
    // (6 + 1) out of a ceiling of 4 * 6.
    assert_eq!(view.progress_percent, 29);
}
