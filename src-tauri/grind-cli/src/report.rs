//! Everything the CLI prints.

use anyhow::Result;
use grind_test_lib::generate::{GenerationEvent, KnowledgeBatchReport, SessionPlan, SessionResult};
use grind_test_lib::vault::quiz::{self, Question, Quiz};
use grind_test_lib::vault::{knowledge, progress, syllabus, Vault};

pub fn event(event: &GenerationEvent) {
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

pub fn batch(report: &KnowledgeBatchReport) {
    println!(
        "\ndone: {} generated, {} skipped, {} failed, ~{} tokens",
        report.generated, report.skipped, report.failed, report.total_tokens
    );
    for error in &report.errors {
        eprintln!("  ! {error}");
    }
}

pub fn session(plan: &SessionPlan) {
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
    questions(&plan.quiz);
}

pub fn session_result(result: &SessionResult) {
    println!("overall: {}%", result.overall_score);
    for outcome in &result.topics {
        println!(
            "\n{} — {}  (open {:?}, quiz {}/{})\n  level {} [{:?}], next review {}",
            outcome.topic_id,
            if outcome.skipped {
                "skipped".to_string()
            } else {
                format!("{}%", outcome.score)
            },
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

pub fn quiz(quiz: &Quiz) {
    println!("{} — {} questions", quiz.id, quiz.questions.len());
    questions(&quiz.questions);
}

/// Correct options are starred: the CLI exists to check what the generator produced.
fn questions(questions: &[Question]) {
    for question in questions {
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

pub fn status(vault: &Vault) -> Result<()> {
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
            quiz::list(vault, &subject.id)?.len(),
            stats.accuracy_percent,
        );
    }
    Ok(())
}

pub fn topics(vault: &Vault, subject_id: &str, missing_only: bool) -> Result<()> {
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
