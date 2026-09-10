//! One knowledge note per topic, at the smart tier.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::KNOWLEDGE_TIMEOUT;
use crate::agy::{self, ModelTier};
use crate::vault::knowledge::{self, KnowledgeNote};
use crate::vault::syllabus::{Subject, Topic};
use crate::vault::Vault;

const KNOWLEDGE_PROMPT: &str = include_str!("../../prompts/knowledge.md");
const KNOWLEDGE_SCHEMA: &str = include_str!("../../schemas/knowledge.json");

#[derive(Debug, Deserialize)]
struct KeyTerm {
    term: String,
    definition: String,
}

#[derive(Debug, Deserialize)]
struct KnowledgeDraft {
    summary: String,
    /// The topic answered in one paragraph. Defaulted because notes predate the field.
    #[serde(default)]
    exam_answer: String,
    markdown: String,
    #[serde(default)]
    key_terms: Vec<KeyTerm>,
    #[serde(default)]
    exam_traps: Vec<String>,
}

impl KnowledgeDraft {
    /// Flatten the structured draft into the markdown body stored on disk.
    fn into_body(self) -> String {
        let mut body = String::new();
        body.push_str(self.summary.trim());
        body.push_str("\n\n");
        // Directly under the summary, and above the note proper, because this is what the
        // condensed reading leads with — see `knowledge::digest`.
        if !self.exam_answer.trim().is_empty() {
            body.push_str(knowledge::CORE_HEADING);
            body.push_str("\n\n");
            body.push_str(self.exam_answer.trim());
            body.push_str("\n\n");
        }
        body.push_str(self.markdown.trim());

        if !self.key_terms.is_empty() {
            body.push_str("\n\n## Ключові терміни\n\n");
            for term in &self.key_terms {
                body.push_str(&format!(
                    "- **{}** — {}\n",
                    term.term.trim(),
                    term.definition.trim()
                ));
            }
        }
        if !self.exam_traps.is_empty() {
            body.push_str("\n## На чому підловлюють\n\n");
            for trap in &self.exam_traps {
                body.push_str(&format!("- {}\n", trap.trim()));
            }
        }
        body
    }
}

fn knowledge_prompt(subject: &Subject, topic: &Topic) -> String {
    let siblings = subject
        .sections
        .iter()
        .find(|s| s.index == topic.section)
        .map(|section| {
            section
                .topics
                .iter()
                .filter(|t| t.id != topic.id)
                .map(|t| format!("- {}", t.title))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    // A nested topic reads as a fragment without the group heading above it — a subtopic
    // named only "Types of nodes" means nothing until you know which group it sits under.
    let group = topic
        .group
        .as_deref()
        .map(|group| format!("\nGroup within the section: {group}"))
        .unwrap_or_default();

    KNOWLEDGE_PROMPT
        .replace("{{SUBJECT_TITLE}}", &subject.title)
        .replace("{{SECTION_TITLE}}", &topic.section_title)
        .replace("{{GROUP}}", &group)
        .replace("{{TOPIC_TITLE}}", &topic.title)
        .replace(
            "{{SIBLING_TOPICS}}",
            if siblings.is_empty() {
                "(none)"
            } else {
                &siblings
            },
        )
}

/// Generate and persist the note for a single topic.
pub async fn knowledge_for_topic(
    vault: &Vault,
    subject: &Subject,
    topic: &Topic,
) -> Result<(KnowledgeNote, agy::Usage)> {
    let prompt = knowledge_prompt(subject, topic);
    let response = agy::run::<KnowledgeDraft>(
        &prompt,
        ModelTier::Smart,
        KNOWLEDGE_SCHEMA,
        KNOWLEDGE_TIMEOUT,
    )
    .await
    .with_context(|| format!("generating knowledge for {}", topic.id))?;

    let body = response.data.into_body();
    if body.trim().len() < 200 {
        bail!("model returned a suspiciously short note for {}", topic.id);
    }
    let note = knowledge::write(vault, topic, &response.model, &body)?;
    Ok((note, response.usage))
}
