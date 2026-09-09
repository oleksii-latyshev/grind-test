use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::paths::Vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionKind {
    Single,
    Multi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    /// Let the generator mix all three.
    Mixed,
}

impl Difficulty {
    pub fn as_prompt(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy — recall of definitions and basic facts",
            Difficulty::Medium => "medium — understanding, comparison and application",
            Difficulty::Hard => "hard — subtle distinctions, edge cases and multi-step reasoning",
            Difficulty::Mixed => "mixed — roughly one third easy, one third medium, one third hard",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub topic_id: String,
    #[serde(rename = "type")]
    pub kind: QuestionKind,
    pub question: String,
    pub options: Vec<String>,
    /// 0-based indices into `options`.
    pub correct: Vec<usize>,
    pub explanation: String,
    pub difficulty: Difficulty,
}

impl Question {
    pub fn is_correct(&self, selected: &[usize]) -> bool {
        let mut expected = self.correct.clone();
        let mut got = selected.to_vec();
        expected.sort_unstable();
        expected.dedup();
        got.sort_unstable();
        got.dedup();
        expected == got
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quiz {
    pub id: String,
    pub subject: String,
    pub title: String,
    pub created_at: String,
    pub model: String,
    pub difficulty: Difficulty,
    pub topic_ids: Vec<String>,
    pub questions: Vec<Question>,
}

/// Enough to render a quiz list without loading every question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSummary {
    pub id: String,
    pub subject: String,
    pub title: String,
    pub created_at: String,
    pub question_count: usize,
    pub difficulty: Difficulty,
}

impl From<&Quiz> for QuizSummary {
    fn from(quiz: &Quiz) -> Self {
        Self {
            id: quiz.id.clone(),
            subject: quiz.subject.clone(),
            title: quiz.title.clone(),
            created_at: quiz.created_at.clone(),
            question_count: quiz.questions.len(),
            difficulty: quiz.difficulty,
        }
    }
}

pub fn save(vault: &Vault, quiz: &Quiz) -> Result<()> {
    let dir = vault.quizzes_dir(&quiz.subject);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{}.json", quiz.id));
    let json = serde_json::to_string_pretty(quiz)?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn read(vault: &Vault, subject: &str, id: &str) -> Result<Quiz> {
    let path = vault.quizzes_dir(subject).join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading quiz {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing quiz {}", path.display()))
}

/// Newest first. Unreadable files are skipped rather than failing the whole listing —
/// a half-written quiz must not make the library unusable.
pub fn list(vault: &Vault, subject: &str) -> Result<Vec<QuizSummary>> {
    let dir = vault.quizzes_dir(subject);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(quiz) = serde_json::from_str::<Quiz>(&raw) {
            out.push(QuizSummary::from(&quiz));
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// Question stems from the most recent quizzes, fed back into generation so the model
/// stops asking the same thing every time.
pub fn recent_question_texts(vault: &Vault, subject: &str, limit: usize) -> Vec<String> {
    let mut summaries = match list(vault, subject) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    summaries.truncate(5);
    let mut out = Vec::new();
    for summary in summaries {
        if let Ok(quiz) = read(vault, subject, &summary.id) {
            for question in quiz.questions {
                out.push(question.question);
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(correct: Vec<usize>) -> Question {
        Question {
            id: "q1".into(),
            topic_id: "demo/1.1".into(),
            kind: QuestionKind::Multi,
            question: "?".into(),
            options: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            correct,
            explanation: String::new(),
            difficulty: Difficulty::Medium,
        }
    }

    #[test]
    fn grading_ignores_order_and_duplicates() {
        let q = question(vec![2, 0]);
        assert!(q.is_correct(&[0, 2]));
        assert!(q.is_correct(&[2, 0, 0]));
        assert!(!q.is_correct(&[0]));
        assert!(!q.is_correct(&[0, 1, 2]));
    }
}
