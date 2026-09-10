//! On-demand hints for a quiz question, grounded in the note for its topic.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::truncate_chars;
use crate::agy::{self, ModelTier};
use crate::vault::{knowledge, quiz, syllabus, Vault};

const HINT_PROMPT: &str = include_str!("../../prompts/hint.md");
const HINT_SCHEMA: &str = include_str!("../../schemas/hint.json");
const HINT_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hint {
    pub hint: String,
    #[serde(default)]
    pub related_concepts: Vec<String>,
}

pub async fn hint(
    vault: &Vault,
    subject_id: &str,
    quiz_id: &str,
    question_id: &str,
) -> Result<Hint> {
    let quiz = quiz::read(vault, subject_id, quiz_id)?;
    let question = quiz
        .questions
        .iter()
        .find(|q| q.id == question_id)
        .ok_or_else(|| anyhow!("question '{question_id}' not found in quiz '{quiz_id}'"))?;

    let subject = syllabus::load(vault, subject_id)?;
    let note = subject
        .find_topic(&question.topic_id)
        .map(|topic| knowledge::read(vault, topic))
        .transpose()?
        .flatten();

    let knowledge_block = match note {
        Some(note) => truncate_chars(&note.body, 6000),
        None => "(конспект для цієї теми ще не згенеровано)".to_string(),
    };

    let options = question
        .options
        .iter()
        .enumerate()
        .map(|(i, option)| format!("{}. {}", i + 1, option))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = HINT_PROMPT
        .replace("{{QUESTION}}", &question.question)
        .replace("{{OPTIONS}}", &options)
        .replace("{{KNOWLEDGE}}", &knowledge_block);

    let response = agy::run::<Hint>(&prompt, ModelTier::Smart, HINT_SCHEMA, HINT_TIMEOUT).await?;
    Ok(response.data)
}
