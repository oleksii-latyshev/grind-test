//! Terminal front-end over the same vault code the app uses.
//!
//! Bulk knowledge generation takes hours; running it here keeps it out of the GUI.

mod report;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use grind_test_lib::generate::{self, QuizRequest, Selection, SessionMode};
use grind_test_lib::vault::paths::resolve_vault_root;
use grind_test_lib::vault::quiz::Difficulty;
use grind_test_lib::vault::{knowledge, syllabus, Vault};

/// Read the tier→model mapping the app persisted. The CLI has no Tauri path resolver, so it
/// looks in the same place `dirs` would; failing to find it just means the defaults.
fn load_model_settings() -> grind_test_lib::agy::ModelSettings {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Default::default();
    };
    let config_dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/com.user.grind-test")
    } else {
        home.join(".config/com.user.grind-test")
    };
    grind_test_lib::config::load(&config_dir).models
}

#[derive(Parser)]
#[command(name = "grind", about = "Vault tooling for grind-test")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the vault location and what is in it.
    Status,
    /// List a subject's topics and whether each has a knowledge note.
    Topics {
        #[arg(short, long)]
        subject: String,
        /// Only topics that still need a note.
        #[arg(long)]
        missing: bool,
    },
    /// Generate the missing knowledge notes for a subject (smart model).
    Knowledge {
        #[arg(short, long)]
        subject: String,
        /// Specific topic ids, e.g. `demo/2.7`. Defaults to the whole subject.
        #[arg(short, long = "topic")]
        topics: Vec<String>,
        /// Stop after this many topics — useful for a cheap smoke test.
        #[arg(long)]
        limit: Option<usize>,
        /// Regenerate notes that already exist.
        #[arg(long)]
        force: bool,
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
    },
    /// Start one study session and print it (does not record anything).
    Session {
        #[arg(short, long)]
        subject: String,
        #[arg(long, default_value_t = 3)]
        size: usize,
        /// Study these topic ids instead of what the scheduler would serve.
        #[arg(short, long = "topic")]
        topics: Vec<String>,
        /// Pace: full notes and an essay, full notes and a list, or condensed notes.
        #[arg(short, long, value_enum, default_value_t = ModeArg::Full)]
        mode: ModeArg,
        /// Draw topics from across the syllabus, the way an exam paper does.
        #[arg(long)]
        spread: bool,
    },
    /// Grade a session started with `session`, from a JSON file of answers.
    ///
    /// The file looks like `{"open": {"demo/1.1": "..."}, "quiz": {"q1": [0], "q2": [1, 3]}}`.
    SessionFinish {
        #[arg(short, long)]
        subject: String,
        #[arg(long)]
        session: String,
        #[arg(long)]
        answers: PathBuf,
    },
    /// Generate one quiz (fast model).
    Quiz {
        #[arg(short, long)]
        subject: String,
        #[arg(short, long, default_value_t = 10)]
        count: usize,
        #[arg(short, long, value_enum, default_value_t = DifficultyArg::Mixed)]
        difficulty: DifficultyArg,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum ModeArg {
    Full,
    Balanced,
    Sprint,
}

impl From<ModeArg> for SessionMode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Full => SessionMode::Full,
            ModeArg::Balanced => SessionMode::Balanced,
            ModeArg::Sprint => SessionMode::Sprint,
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum DifficultyArg {
    Easy,
    Medium,
    Hard,
    Mixed,
}

impl From<DifficultyArg> for Difficulty {
    fn from(value: DifficultyArg) -> Self {
        match value {
            DifficultyArg::Easy => Difficulty::Easy,
            DifficultyArg::Medium => Difficulty::Medium,
            DifficultyArg::Hard => Difficulty::Hard,
            DifficultyArg::Mixed => Difficulty::Mixed,
        }
    }
}

#[derive(serde::Deserialize, Default)]
struct AnswerFile {
    #[serde(default)]
    open: BTreeMap<String, String>,
    #[serde(default)]
    quiz: BTreeMap<String, Vec<usize>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // The CLI shares the app's model settings; without this it would silently run the
    // shipped defaults while the GUI used the student's picks.
    grind_test_lib::agy::configure(load_model_settings());
    let vault = Vault::new(resolve_vault_root(None, None));

    match cli.command {
        Command::Status => report::status(&vault)?,
        Command::Topics { subject, missing } => report::topics(&vault, &subject, missing)?,
        Command::Knowledge {
            subject,
            topics,
            limit,
            force,
            concurrency,
        } => {
            let selection = resolve_selection(&vault, &subject, topics, limit, force)?;
            let sink: generate::EventSink = Arc::new(|event| report::event(&event));
            let batch =
                generate::knowledge_batch(&vault, &subject, selection, force, concurrency, sink)
                    .await?;
            report::batch(&batch);
        }
        Command::Session {
            subject,
            size,
            topics,
            mode,
            spread,
        } => {
            let chosen = (!topics.is_empty()).then_some(topics);
            let selection = if spread {
                Selection::Spread
            } else {
                Selection::Scheduled
            };
            let plan =
                generate::start_session(&vault, &subject, size, chosen, mode.into(), selection)
                    .await?;
            report::session(&plan);
        }
        Command::SessionFinish {
            subject,
            session,
            answers,
        } => {
            let raw = std::fs::read_to_string(&answers)
                .with_context(|| format!("reading {}", answers.display()))?;
            let file: AnswerFile = serde_json::from_str(&raw).context("parsing answers file")?;
            let result =
                generate::finish_session(&vault, &session, &subject, file.open, file.quiz).await?;
            report::session_result(&result);
        }
        Command::Quiz {
            subject,
            count,
            difficulty,
        } => {
            let quiz = generate::quiz(
                &vault,
                QuizRequest {
                    subject,
                    topic_ids: None,
                    question_count: count,
                    difficulty: difficulty.into(),
                },
            )
            .await?;
            report::quiz(&quiz);
        }
    }
    Ok(())
}

/// `--limit` needs the topic list up front, so turn every selection into explicit ids.
fn resolve_selection(
    vault: &Vault,
    subject_id: &str,
    topics: Vec<String>,
    limit: Option<usize>,
    force: bool,
) -> Result<Option<Vec<String>>> {
    if limit.is_none() {
        return Ok(if topics.is_empty() {
            None
        } else {
            Some(topics)
        });
    }
    let subject = syllabus::load(vault, subject_id)?;
    let selected: Vec<String> = subject
        .topics()
        .filter(|topic| topics.is_empty() || topics.contains(&topic.id))
        .filter(|topic| force || !knowledge::exists(vault, topic))
        .take(limit.unwrap_or(usize::MAX))
        .map(|topic| topic.id.clone())
        .collect();
    Ok(Some(selected))
}
