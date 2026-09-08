//! Terminal front-end over the same vault code the app uses.
//!
//! Bulk knowledge generation takes hours; running it here keeps it out of the GUI.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use grind_test_lib::generate::{self, GenerationEvent, QuizRequest, Selection, SessionMode};
use grind_test_lib::vault::paths::resolve_vault_root;
use grind_test_lib::vault::progress;
use grind_test_lib::vault::quiz::Difficulty;
use grind_test_lib::vault::{knowledge, syllabus, Vault};

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
        /// Specific topic ids, e.g. `f3/2.7`. Defaults to the whole subject.
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
    /// The file looks like `{"open": {"f7/1.1": "..."}, "quiz": {"q1": [0], "q2": [1, 3]}}`.
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let vault = Vault::new(resolve_vault_root(None, None));

    match cli.command {
        Command::Status => status(&vault)?,
        Command::Topics { subject, missing } => topics(&vault, &subject, missing)?,
        Command::Knowledge {
            subject,
            topics,
            limit,
            force,
            concurrency,
        } => {
            let selection = resolve_selection(&vault, &subject, topics, limit, force)?;
            let sink: generate::EventSink = Arc::new(|event| print_event(&event));
            let report =
                generate::knowledge_batch(&vault, &subject, selection, force, concurrency, sink)
                    .await?;
            println!(
                "\ndone: {} generated, {} skipped, {} failed, ~{} tokens",
                report.generated, report.skipped, report.failed, report.total_tokens
            );
            for error in &report.errors {
                eprintln!("  ! {error}");
            }
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
            println!("session {} — {} topics", plan.id, plan.topics.len());
            for entry in &plan.topics {
                println!(
                    "  {} {} [{:?}, level {}]",
                    entry.topic.id, entry.topic.title, entry.stage, entry.level
                );
            }
            println!("\n-- open questions --");
            for question in &plan.open_questions {
                println!("\n[{}] {}", question.topic_id, question.question);
                for point in &question.expected_points {
                    println!("    · {point}");
                }
            }
            println!("\n-- quiz ({} questions) --", plan.quiz.len());
            for question in &plan.quiz {
                println!("\n[{}] {}", question.topic_id, question.question);
                for (index, option) in question.options.iter().enumerate() {
                    let mark = if question.correct.contains(&index) {
                        "*"
                    } else {
                        " "
                    };
                    println!("  {mark} {}. {option}", index + 1);
                }
            }
        }
        Command::SessionFinish {
            subject,
            session,
            answers,
        } => {
            #[derive(serde::Deserialize, Default)]
            struct AnswerFile {
                #[serde(default)]
                open: BTreeMap<String, String>,
                #[serde(default)]
                quiz: BTreeMap<String, Vec<usize>>,
            }

            let raw = std::fs::read_to_string(&answers)
                .with_context(|| format!("reading {}", answers.display()))?;
            let file: AnswerFile = serde_json::from_str(&raw).context("parsing answers file")?;

            let result =
                generate::finish_session(&vault, &session, &subject, file.open, file.quiz).await?;

            println!("overall: {}%", result.overall_score);
            for outcome in &result.topics {
                println!(
                    "\n{} — {}%  (open {:?}, quiz {}/{})\n  level {} [{:?}], next review {}",
                    outcome.topic_id,
                    outcome.score,
                    outcome.open_score,
                    outcome.quiz_correct,
                    outcome.quiz_total,
                    outcome.level,
                    outcome.stage,
                    outcome.due_at.as_deref().unwrap_or("-"),
                );
            }
            for grading in &result.gradings {
                println!(
                    "\n[{}] {}%\n  {}",
                    grading.topic_id, grading.score, grading.verdict
                );
                for point in &grading.missed {
                    println!("  – пропущено: {point}");
                }
            }
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
            println!("{} — {} questions", quiz.id, quiz.questions.len());
            for question in &quiz.questions {
                println!("\n[{}] {}", question.topic_id, question.question);
                for (index, option) in question.options.iter().enumerate() {
                    let mark = if question.correct.contains(&index) {
                        "*"
                    } else {
                        " "
                    };
                    println!("  {mark} {}. {option}", index + 1);
                }
            }
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

fn print_event(event: &GenerationEvent) {
    match event {
        GenerationEvent::Started {
            subject,
            total,
            skipped,
        } => {
            println!("{subject}: generating {total} notes ({skipped} already present)");
        }
        GenerationEvent::TopicStarted { topic_id, .. } => println!("  → {topic_id}"),
        GenerationEvent::TopicDone {
            topic_id,
            done,
            total,
            ..
        } => {
            println!("  ✓ {topic_id}  [{done}/{total}]");
        }
        GenerationEvent::TopicFailed {
            topic_id, error, ..
        } => {
            eprintln!("  ✗ {topic_id}: {error}");
        }
        GenerationEvent::Finished { .. } => {}
    }
}

fn status(vault: &Vault) -> Result<()> {
    println!("vault: {}", vault.root.display());
    match grind_test_lib::agy::resolve_binary() {
        Ok(path) => println!("agy:   {}", path.display()),
        Err(error) => println!("agy:   NOT FOUND ({error})"),
    }
    let mastery = progress::load_mastery(vault);
    println!();
    for subject in syllabus::load_all(vault)? {
        let topics: Vec<_> = subject.topics().collect();
        let with_knowledge = topics
            .iter()
            .filter(|t| knowledge::exists(vault, t))
            .count();
        let stats = progress::subject_stats(&mastery, &subject.id, &topics, with_knowledge);
        println!(
            "{:<4} {:>3} topics  {:>3} notes  {:>3} quizzes  {:>3}% accuracy",
            subject.id,
            topics.len(),
            with_knowledge,
            grind_test_lib::vault::quiz::list(vault, &subject.id)?.len(),
            stats.accuracy_percent,
        );
    }
    Ok(())
}

fn topics(vault: &Vault, subject_id: &str, missing_only: bool) -> Result<()> {
    let subject = syllabus::load(vault, subject_id)?;
    for section in &subject.sections {
        println!("\n## {}", section.title);
        for topic in &section.topics {
            let has = knowledge::exists(vault, topic);
            if missing_only && has {
                continue;
            }
            println!(
                "{} {:<10} {}",
                if has { "✓" } else { " " },
                topic.id,
                topic.title
            );
        }
    }
    Ok(())
}
