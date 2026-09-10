//! The question shape the quiz and session generators share, and the check that drops what
//! the model got structurally wrong.

use serde::Deserialize;

use crate::vault::quiz::{Difficulty, Question, QuestionKind};

#[derive(Debug, Deserialize)]
pub(super) struct QuestionDraft {
    topic_id: String,
    #[serde(rename = "type")]
    kind: QuestionKind,
    question: String,
    options: Vec<String>,
    correct: Vec<usize>,
    explanation: String,
    difficulty: DraftDifficulty,
}

/// The generator only ever labels a question easy/medium/hard; `mixed` is a request, not a
/// property of a question.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DraftDifficulty {
    Easy,
    Medium,
    Hard,
}

impl From<DraftDifficulty> for Difficulty {
    fn from(value: DraftDifficulty) -> Self {
        match value {
            DraftDifficulty::Easy => Difficulty::Easy,
            DraftDifficulty::Medium => Difficulty::Medium,
            DraftDifficulty::Hard => Difficulty::Hard,
        }
    }
}

/// Drop questions the model got structurally wrong rather than shipping a quiz that cannot
/// be answered correctly.
pub(super) fn sanitize_questions(
    drafts: Vec<QuestionDraft>,
    topic_ids: &[String],
) -> Vec<Question> {
    let fallback = topic_ids.first().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for (index, draft) in drafts.into_iter().enumerate() {
        if draft.options.len() < 2 || draft.question.trim().is_empty() {
            continue;
        }
        let mut correct: Vec<usize> = draft
            .correct
            .into_iter()
            .filter(|i| *i < draft.options.len())
            .collect();
        correct.sort_unstable();
        correct.dedup();
        if correct.is_empty() || correct.len() == draft.options.len() {
            continue;
        }
        // Trust the shape of the answer over the model's own `type` label.
        let kind = if correct.len() > 1 {
            QuestionKind::Multi
        } else {
            QuestionKind::Single
        };
        let _ = draft.kind;

        out.push(Question {
            id: format!("q{}", index + 1),
            topic_id: if topic_ids.contains(&draft.topic_id) {
                draft.topic_id
            } else {
                fallback.clone()
            },
            kind,
            question: draft.question.trim().to_string(),
            options: draft
                .options
                .into_iter()
                .map(|o| o.trim().to_string())
                .collect(),
            correct,
            explanation: draft.explanation.trim().to_string(),
            difficulty: draft.difficulty.into(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(options: Vec<&str>, correct: Vec<usize>) -> QuestionDraft {
        QuestionDraft {
            topic_id: "demo/1.1".into(),
            kind: QuestionKind::Single,
            question: "Питання?".into(),
            options: options.into_iter().map(String::from).collect(),
            correct,
            explanation: "бо так".into(),
            difficulty: DraftDifficulty::Medium,
        }
    }

    #[test]
    fn drops_unanswerable_questions() {
        let topics = vec!["demo/1.1".to_string()];
        let kept = sanitize_questions(
            vec![
                draft(vec!["a", "b", "c", "d"], vec![1]),
                draft(vec!["a", "b", "c", "d"], vec![]), // no correct option
                draft(vec!["a", "b", "c", "d"], vec![0, 1, 2, 3]), // every option correct
                draft(vec!["a"], vec![0]),               // not a choice
                draft(vec!["a", "b", "c", "d"], vec![9]), // out of range
            ],
            &topics,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "q1");
    }

    #[test]
    fn infers_kind_from_the_answer_shape() {
        let topics = vec!["demo/1.1".to_string()];
        let kept = sanitize_questions(vec![draft(vec!["a", "b", "c", "d"], vec![0, 2])], &topics);
        assert!(matches!(kept[0].kind, QuestionKind::Multi));
    }

    #[test]
    fn unknown_topic_ids_fall_back_to_a_real_one() {
        let topics = vec!["demo/1.1".to_string()];
        let mut d = draft(vec!["a", "b", "c", "d"], vec![0]);
        d.topic_id = "made/up".into();
        let kept = sanitize_questions(vec![d], &topics);
        assert_eq!(kept[0].topic_id, "demo/1.1");
    }
}
