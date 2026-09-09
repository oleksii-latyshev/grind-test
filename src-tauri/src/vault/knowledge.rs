use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::paths::Vault;
use super::syllabus::Topic;

/// A generated knowledge note for one syllabus topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNote {
    pub topic_id: String,
    pub subject: String,
    pub title: String,
    pub section_title: String,
    pub model: String,
    pub generated_at: String,
    pub body: String,
    pub path: String,
}

/// Front matter + body, written so a human can read the file in any markdown editor.
fn render(note: &KnowledgeNote) -> String {
    format!(
        "---\ntopic_id: {}\nsubject: {}\nsection: {}\ntitle: {}\nmodel: {}\ngenerated_at: {}\n---\n\n{}\n",
        note.topic_id,
        note.subject,
        yaml_escape(&note.section_title),
        yaml_escape(&note.title),
        note.model,
        note.generated_at,
        note.body.trim()
    )
}

fn yaml_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn yaml_unescape(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        trimmed.to_string()
    }
}

/// Locate an existing note by its `SS.TT-` filename prefix, so a reworded topic title
/// (and therefore a changed slug) still resolves to the note already on disk.
pub fn find_path(vault: &Vault, subject: &str, section: usize, index: usize) -> Option<PathBuf> {
    let dir = vault.knowledge_dir(subject);
    let prefix = format!("{section:02}.{index:02}-");
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_str()?;
        if name.starts_with(&prefix) && name.ends_with(".md") {
            return Some(path);
        }
    }
    None
}

pub fn exists(vault: &Vault, topic: &Topic) -> bool {
    find_path(vault, &topic.subject, topic.section, topic.index).is_some()
}

pub fn read(vault: &Vault, topic: &Topic) -> Result<Option<KnowledgeNote>> {
    let Some(path) = find_path(vault, &topic.subject, topic.section, topic.index) else {
        return Ok(None);
    };
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(parse(&raw, &path)))
}

fn parse(raw: &str, path: &PathBuf) -> KnowledgeNote {
    let mut note = KnowledgeNote {
        topic_id: String::new(),
        subject: String::new(),
        title: String::new(),
        section_title: String::new(),
        model: String::new(),
        generated_at: String::new(),
        body: String::new(),
        path: path.to_string_lossy().to_string(),
    };

    let body = match raw.strip_prefix("---\n") {
        Some(rest) => match rest.split_once("\n---") {
            Some((front, body)) => {
                for line in front.lines() {
                    let Some((key, value)) = line.split_once(':') else {
                        continue;
                    };
                    let value = yaml_unescape(value);
                    match key.trim() {
                        "topic_id" => note.topic_id = value,
                        "subject" => note.subject = value,
                        "title" => note.title = value,
                        "section" => note.section_title = value,
                        "model" => note.model = value,
                        "generated_at" => note.generated_at = value,
                        _ => {}
                    }
                }
                body.trim_start_matches('\n')
            }
            None => raw,
        },
        None => raw,
    };
    note.body = body.trim().to_string();
    note
}

/// The topic answered in a single paragraph — the substance an examiner is listening for,
/// not a description of what the topic covers.
pub const CORE_HEADING: &str = "## Головне";

/// Headings the generator always writes, and the only ones a fast revision pass needs.
const DIGEST_SECTIONS: [&str; 3] = [
    CORE_HEADING,
    "## Ключові терміни",
    "## На чому підловлюють",
];

/// A short version of a note for fast revision: the opening summary, the one-paragraph
/// answer, and the key-terms and exam-traps sections.
///
/// No model call is involved — the generator already produced this material as part of every
/// note, so a sprint costs nothing extra to prepare. A note written before `## Головне`
/// existed simply contributes no such section, and its summary still leads.
pub fn digest(body: &str) -> String {
    let sections: Vec<String> = DIGEST_SECTIONS
        .iter()
        .filter_map(|heading| extract_section(body, heading))
        .collect();

    // No heading at all means the note was not written by our generator, so there is
    // nothing to condense down to; a trimmed full note beats a two-sentence summary.
    if sections.is_empty() {
        return truncate(body, 2200);
    }

    let lead = body
        .lines()
        .take_while(|line| !line.trim_start().starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    let mut parts: Vec<String> = Vec::new();
    if !lead.is_empty() {
        parts.push(lead);
    }
    parts.extend(sections);
    parts.join("\n\n")
}

fn extract_section(body: &str, heading: &str) -> Option<String> {
    let start = body.find(heading)?;
    let rest = &body[start..];
    let end = rest[heading.len()..]
        .find("\n## ")
        .map(|offset| offset + heading.len())
        .unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_string(),
        Some((cut, _)) => format!("{}\n…", &text[..cut]),
    }
}

pub fn write(vault: &Vault, topic: &Topic, model: &str, body: &str) -> Result<KnowledgeNote> {
    let dir = vault.knowledge_dir(&topic.subject);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    // A retitled topic changes the slug; drop the stale file so we never keep two notes
    // for the same topic.
    if let Some(old) = find_path(vault, &topic.subject, topic.section, topic.index) {
        let _ = std::fs::remove_file(old);
    }

    let path = vault.knowledge_file(&topic.subject, topic.section, topic.index, &topic.title);
    let note = KnowledgeNote {
        topic_id: topic.id.clone(),
        subject: topic.subject.clone(),
        title: topic.title.clone(),
        section_title: topic.section_title.clone(),
        model: model.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        body: body.trim().to_string(),
        path: path.to_string_lossy().to_string(),
    };
    std::fs::write(&path, render(&note)).with_context(|| format!("writing {}", path.display()))?;
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_NOTE: &str = "Стисла суть теми у двох реченнях, якої достатньо для орієнтації.\n\n## Головне\n\nВідповідь одним абзацом, яку студент написав би на екзамені.\n\n## Розділ\n\nДовгий текст.\n\n## Ще розділ\n\nЩе довший текст.\n\n## Ключові терміни\n\n- **Термін** — визначення терміна, достатньо докладне для повторення.\n- **Інший** — ще одне визначення, яке також займає місце.\n\n## На чому підловлюють\n\n- Плутають одне з іншим, і це найчастіша помилка на екзамені.\n- Забувають про третій випадок, який завжди запитують окремо.\n";

    #[test]
    fn digest_keeps_the_summary_and_the_revision_sections() {
        let short = digest(FULL_NOTE);

        assert!(short.starts_with("Стисла суть теми"));
        assert!(short.contains("## Головне"));
        assert!(short.contains("Відповідь одним абзацом"));
        assert!(short.contains("## Ключові терміни"));
        assert!(short.contains("## На чому підловлюють"));
        // The bulk of the note — the sections meant for a first read — is dropped.
        assert!(!short.contains("Довгий текст"));
        assert!(!short.contains("Ще довший текст"));
        assert!(short.len() < FULL_NOTE.len());
    }

    #[test]
    fn digest_leads_with_the_one_paragraph_answer() {
        let short = digest(FULL_NOTE);
        let core = short.find("## Головне").unwrap();
        assert!(core < short.find("## Ключові терміни").unwrap());
    }

    #[test]
    fn digest_of_a_note_written_before_the_answer_existed_still_works() {
        let older = FULL_NOTE.replace(
            "## Головне\n\nВідповідь одним абзацом, яку студент написав би на екзамені.\n\n",
            "",
        );
        let short = digest(&older);
        assert!(short.starts_with("Стисла суть теми"));
        assert!(!short.contains("## Головне"));
        assert!(short.contains("## Ключові терміни"));
    }

    #[test]
    fn digest_falls_back_to_the_note_when_the_sections_are_missing() {
        let plain = format!("Коротка суть.\n\n## Розділ\n\n{}", "текст ".repeat(200));
        let short = digest(&plain);
        assert!(short.contains("Розділ"));
        assert!(short.contains("текст"));
    }

    #[test]
    fn round_trips_front_matter() {
        let note = KnowledgeNote {
            topic_id: "f3/2.7".into(),
            subject: "f3".into(),
            title: "Моделі подання знань: \"фрейми\"".into(),
            section_title: "Штучний інтелект".into(),
            model: "gemini-3.1-pro-high".into(),
            generated_at: "2026-09-07T00:00:00Z".into(),
            body: "## Суть\n\nТекст.".into(),
            path: String::new(),
        };
        let parsed = parse(&render(&note), &PathBuf::from("x.md"));
        assert_eq!(parsed.topic_id, note.topic_id);
        assert_eq!(parsed.title, note.title);
        assert_eq!(parsed.body, note.body);
    }
}
