use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::paths::Vault;

/// One exam topic, e.g. `demo/2.7`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub subject: String,
    /// 1-based index of the owning section within the syllabus file.
    pub section: usize,
    pub section_title: String,
    /// Position within the section, 1-based. For a flat section this is the number printed
    /// in the syllabus; for a nested one the printed numbering repeats, so position is what
    /// actually identifies a topic.
    pub index: usize,
    pub title: String,
    /// Heading of the numbered group a nested topic belongs to, e.g. "Перша група".
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub index: usize,
    pub title: String,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    /// Syllabus filename without extension, e.g. `demo` for `syllabus/demo.md`.
    pub id: String,
    pub title: String,
    pub sections: Vec<Section>,
}

impl Subject {
    pub fn topics(&self) -> impl Iterator<Item = &Topic> {
        self.sections.iter().flat_map(|s| s.topics.iter())
    }

    pub fn topic_count(&self) -> usize {
        self.sections.iter().map(|s| s.topics.len()).sum()
    }

    pub fn find_topic(&self, id: &str) -> Option<&Topic> {
        self.topics().find(|t| t.id == id)
    }
}

/// Parse every `vault/syllabus/*.md`, sorted by subject id.
pub fn load_all(vault: &Vault) -> Result<Vec<Subject>> {
    let dir = vault.syllabus_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut subjects = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        subjects.push(parse(&id, &raw));
    }
    subjects.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(subjects)
}

pub fn load(vault: &Vault, subject_id: &str) -> Result<Subject> {
    let path = vault.syllabus_dir().join(format!("{subject_id}.md"));
    let raw = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "no syllabus for subject '{subject_id}' at {}",
            path.display()
        )
    })?;
    Ok(parse(subject_id, &raw))
}

/// Syllabus grammar: `# Title`, `## Section`, then numbered topic lines.
///
/// Numbering may be nested — a section can list `1.` as a group heading and `1.1.`, `1.2.`
/// beneath it — so the whole numeric prefix is parsed, not just the first component. A line
/// that is neither a heading nor numbered continues the previous entry.
pub fn parse(subject_id: &str, raw: &str) -> Subject {
    let mut title = subject_id.to_string();
    let mut sections: Vec<Section> = Vec::new();
    let mut pending: Vec<RawItem> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("## ") {
            if let Some(section) = sections.last_mut() {
                section.topics = flatten(subject_id, section, std::mem::take(&mut pending));
            }
            sections.push(Section {
                index: sections.len() + 1,
                title: rest.trim().to_string(),
                topics: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            title = rest.trim().to_string();
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if sections.is_empty() {
            continue; // preamble before the first section heading
        }

        match split_numbered(trimmed) {
            Some((path, text)) => pending.push(RawItem {
                depth: path.len(),
                text: text.to_string(),
            }),
            None => {
                if let Some(previous) = pending.last_mut() {
                    previous.text.push(' ');
                    previous.text.push_str(trimmed);
                }
            }
        }
    }

    if let Some(section) = sections.last_mut() {
        section.topics = flatten(subject_id, section, pending);
    }

    Subject {
        id: subject_id.to_string(),
        title,
        sections,
    }
}

/// One numbered line, before we know whether it is a topic or a group heading.
struct RawItem {
    depth: usize,
    text: String,
}

/// Turn the numbered lines of a section into topics.
///
/// A top-level line immediately followed by deeper ones is a group heading, not an exam
/// topic: it is recorded as context on its children rather than becoming a topic of its own.
fn flatten(subject_id: &str, section: &Section, items: Vec<RawItem>) -> Vec<Topic> {
    let mut collected: Vec<(Option<String>, String)> = Vec::new();
    let mut index = 0;

    while index < items.len() {
        let item = &items[index];
        let has_children = items
            .get(index + 1)
            .is_some_and(|next| next.depth > item.depth);

        if item.depth == 1 && has_children {
            let group = item.text.clone();
            index += 1;
            while index < items.len() && items[index].depth > 1 {
                collected.push((Some(group.clone()), items[index].text.clone()));
                index += 1;
            }
            continue;
        }

        collected.push((None, item.text.clone()));
        index += 1;
    }

    collected
        .into_iter()
        .enumerate()
        .map(|(position, (group, text))| {
            let number = position + 1;
            Topic {
                id: format!("{subject_id}/{}.{number}", section.index),
                subject: subject_id.to_string(),
                section: section.index,
                section_title: section.title.clone(),
                index: number,
                title: text,
                group,
            }
        })
        .collect()
}

/// `"12. Текст"` -> `([12], "Текст")`, `"1.3. Текст"` -> `([1, 3], "Текст")`.
///
/// The prefix must end in a dot, so a topic that merely opens with a number ("2020 рік…")
/// is not mistaken for a list item.
fn split_numbered(line: &str) -> Option<(Vec<usize>, &str)> {
    let end = line
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit() || *ch == '.')
        .map(|(offset, ch)| offset + ch.len_utf8())
        .last()?;

    let prefix = &line[..end];
    if !prefix.ends_with('.') {
        return None;
    }
    let text = line[end..].trim();
    if text.is_empty() {
        return None;
    }

    let path: Vec<usize> = prefix
        .trim_end_matches('.')
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    if path.is_empty() {
        return None;
    }
    Some((path, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Invented throughout: these fixtures exercise the grammar, so nothing is gained by
    // borrowing a real syllabus, and a repository is a poor place to keep one.
    const SAMPLE: &str = "# Тестовий силабус\n\n## I. Перший розділ\n\n1. Перша тема.\n2. Друга тема\nз продовженням.\n\n## II. Другий розділ\n\n1. Третя тема.\n";

    const NESTED: &str = "# T\n\n## 1.7 Розділ із групами\n\n1. Перша група\n   1.1. Підтема одна.\n   1.2. Підтема два.\n2. Друга група.\n   2.1. Підтема три.\n";

    #[test]
    fn parses_sections_and_topics() {
        let s = parse("demo", SAMPLE);
        assert_eq!(s.title, "Тестовий силабус");
        assert_eq!(s.sections.len(), 2);
        assert_eq!(s.topic_count(), 3);
        assert_eq!(s.sections[0].topics[1].title, "Друга тема з продовженням.");
        assert_eq!(s.sections[1].topics[0].id, "demo/2.1");
        assert_eq!(s.find_topic("demo/1.1").unwrap().title, "Перша тема.");
    }

    #[test]
    fn ignores_content_before_first_section() {
        let s = parse("demo", "# T\n\n1. orphan\n\n## A\n\n1. kept\n");
        assert_eq!(s.topic_count(), 1);
    }

    /// Nested numbering used to collapse `1.1.`, `1.2.` and `2.1.` onto the ids `demo/1.1`
    /// and `demo/1.2`, so their knowledge notes overwrote each other.
    #[test]
    fn nested_numbering_produces_distinct_topics() {
        let s = parse("demo", NESTED);
        let topics: Vec<&Topic> = s.topics().collect();

        assert_eq!(topics.len(), 3);
        let ids: Vec<&str> = topics.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["demo/1.1", "demo/1.2", "demo/1.3"]);

        // The group heading is context, not a topic of its own.
        assert_eq!(topics[0].title, "Підтема одна.");
        assert_eq!(topics[0].group.as_deref(), Some("Перша група"));
        assert_eq!(topics[2].title, "Підтема три.");
        assert_eq!(topics[2].group.as_deref(), Some("Друга група."));
    }

    #[test]
    fn a_flat_section_keeps_its_printed_numbering() {
        let s = parse("demo", SAMPLE);
        for section in &s.sections {
            for (position, topic) in section.topics.iter().enumerate() {
                assert_eq!(topic.index, position + 1);
                assert!(topic.group.is_none());
            }
        }
    }

    #[test]
    fn a_leading_number_is_not_a_list_marker() {
        assert!(split_numbered("2020 рік був складним").is_none());
        assert_eq!(split_numbered("7. Текст"), Some((vec![7], "Текст")));
        assert_eq!(split_numbered("1.3. Текст"), Some((vec![1, 3], "Текст")));
    }
}
