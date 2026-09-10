//! Filling in a subject's missing notes a few at a time, each written the moment it lands.

use anyhow::Result;
use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::sync::Arc;

use super::knowledge::knowledge_for_topic;
use crate::vault::knowledge;
use crate::vault::syllabus::{self, Topic};
use crate::vault::Vault;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenerationEvent {
    Started {
        subject: String,
        total: usize,
        skipped: usize,
    },
    TopicStarted {
        topic_id: String,
        title: String,
    },
    TopicDone {
        topic_id: String,
        title: String,
        done: usize,
        total: usize,
    },
    TopicFailed {
        topic_id: String,
        title: String,
        error: String,
    },
    Finished {
        generated: usize,
        failed: usize,
        total_tokens: u64,
    },
}

pub type EventSink = Arc<dyn Fn(GenerationEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeBatchReport {
    pub generated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_tokens: u64,
    pub errors: Vec<String>,
}

/// Fill in missing knowledge notes for a subject.
///
/// Every note is written the moment it lands, and topics that already have one are skipped,
/// so an interrupted run costs nothing but the calls that were in flight.
pub async fn knowledge_batch(
    vault: &Vault,
    subject_id: &str,
    topic_ids: Option<Vec<String>>,
    force: bool,
    concurrency: usize,
    on_event: EventSink,
) -> Result<KnowledgeBatchReport> {
    let subject = syllabus::load(vault, subject_id)?;

    let selected: Vec<Topic> = match &topic_ids {
        Some(ids) => subject
            .topics()
            .filter(|t| ids.contains(&t.id))
            .cloned()
            .collect(),
        None => subject.topics().cloned().collect(),
    };

    let total_selected = selected.len();
    let pending: Vec<Topic> = selected
        .into_iter()
        .filter(|topic| force || !knowledge::exists(vault, topic))
        .collect();
    let skipped = total_selected - pending.len();

    on_event(GenerationEvent::Started {
        subject: subject.id.clone(),
        total: pending.len(),
        skipped,
    });

    let total = pending.len();
    let subject = Arc::new(subject);
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let results: Vec<Result<u64, (Topic, String)>> =
        stream::iter(pending.into_iter().map(|topic| {
            let subject = Arc::clone(&subject);
            let on_event = Arc::clone(&on_event);
            let done = Arc::clone(&done);
            let vault = vault.clone();
            async move {
                on_event(GenerationEvent::TopicStarted {
                    topic_id: topic.id.clone(),
                    title: topic.title.clone(),
                });
                match knowledge_for_topic(&vault, &subject, &topic).await {
                    Ok((_, usage)) => {
                        let position = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        on_event(GenerationEvent::TopicDone {
                            topic_id: topic.id.clone(),
                            title: topic.title.clone(),
                            done: position,
                            total,
                        });
                        Ok(usage.total_tokens)
                    }
                    Err(error) => {
                        let message = format!("{error:#}");
                        on_event(GenerationEvent::TopicFailed {
                            topic_id: topic.id.clone(),
                            title: topic.title.clone(),
                            error: message.clone(),
                        });
                        Err((topic, message))
                    }
                }
            }
        }))
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;

    let mut report = KnowledgeBatchReport {
        generated: 0,
        skipped,
        failed: 0,
        total_tokens: 0,
        errors: Vec::new(),
    };
    for result in results {
        match result {
            Ok(tokens) => {
                report.generated += 1;
                report.total_tokens += tokens;
            }
            Err((topic, message)) => {
                report.failed += 1;
                report.errors.push(format!("{}: {}", topic.id, message));
            }
        }
    }

    on_event(GenerationEvent::Finished {
        generated: report.generated,
        failed: report.failed,
        total_tokens: report.total_tokens,
    });
    Ok(report)
}
