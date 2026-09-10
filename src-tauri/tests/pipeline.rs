//! The study pipeline end to end against a real vault on disk. Only `agy` is replaced — by a
//! shell script that answers by schema and logs which model each call asked for.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use grind_test_lib::agy::{self, ModelSettings};
use grind_test_lib::generate::{self, Selection, SessionMode};
use grind_test_lib::vault::{study, Vault};

const SYLLABUS: &str = "# Демо\n\n## I. Перший розділ\n\n1. Перша тема.\n2. Друга тема.\n\n## II. Другий розділ\n\n1. Третя тема.\n";

const FAKE_AGY: &str = r###"#!/bin/sh
while [ $# -gt 0 ]; do
  case "$1" in
    --print) prompt=$2; shift ;;
    --model) model=$2; shift ;;
    --json-schema) schema=$2; shift ;;
  esac
  shift
done
ids=$(printf '%s\n' "$prompt" | sed -n 's#.*topic_id: \([a-z0-9_-]*/[0-9]*\.[0-9]*\).*#\1#p' | sort -u)

each() {
  sep=
  for id in $ids; do
    printf '%s%s' "$sep" "$(printf '%s' "$1" | sed "s#@#$id#g")"
    sep=,
  done
}

if grep -q exam_answer "$schema"; then
  kind=knowledge
  output='{"summary":"Коротко про тему.","exam_answer":"Відповідь одним абзацом.","markdown":"## Механізм\n\nДокладний опис механізму, достатньо довгий, щоб пройти перевірку на підозріло короткий конспект. Ще одне речення для певності.","key_terms":[{"term":"Термін","definition":"Визначення"}],"exam_traps":["Пастка"]}'
elif grep -q expected_points "$schema"; then
  kind=session
  output='{"open_questions":['"$(each '{"topic_id":"@","question":"Розкрийте @","expected_points":["перший","другий"]}')"'],"quiz_questions":['"$(each '{"topic_id":"@","type":"single","question":"Питання @","options":["так","ні","можливо"],"correct":[0],"explanation":"Бо так.","difficulty":"medium"}')"']}'
else
  kind=grade
  output='{"gradings":['"$(each '{"topic_id":"@","score":90,"verdict":"Добре.","covered":["перший"],"missed":["другий"],"correction":""}')"']}'
fi

echo "$model $kind" >> "$(dirname "$0")/calls.log"
printf 'warming up\n{"status":"SUCCESS","structured_output":%s,"usage":{"total_tokens":10}}\n' "$output"
"###;

fn calls(root: &Path) -> Vec<String> {
    let mut calls: Vec<String> = std::fs::read_to_string(root.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    calls.sort();
    calls
}

#[tokio::test]
async fn syllabus_to_graded_session() {
    let root = std::env::temp_dir().join(format!("grind-pipeline-{}", uuid::Uuid::new_v4()));
    let vault = Vault::new(root.clone());
    std::fs::create_dir_all(vault.syllabus_dir()).unwrap();
    std::fs::write(vault.syllabus_dir().join("demo.md"), SYLLABUS).unwrap();

    let agy_bin = root.join("agy");
    std::fs::write(&agy_bin, FAKE_AGY).unwrap();
    std::fs::set_permissions(&agy_bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("GRIND_AGY_BIN", &agy_bin);
    agy::configure(ModelSettings {
        smart: "smart".into(),
        fast: "fast".into(),
    });

    let report = generate::knowledge_batch(&vault, "demo", None, false, 2, Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!(
        (report.generated, report.failed),
        (3, 0),
        "{:?}",
        report.errors
    );

    let rerun = generate::knowledge_batch(&vault, "demo", None, false, 2, Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!((rerun.generated, rerun.skipped), (0, 3));

    let planned = generate::plan_session(
        &vault,
        "demo",
        2,
        None,
        SessionMode::Full,
        Selection::Scheduled,
    )
    .unwrap();
    assert!(planned.quiz.is_empty());
    assert_eq!(generate::list_unfinished(&vault, "demo").len(), 1);

    let plan = generate::prepare_questions(&vault, "demo", &planned.id)
        .await
        .unwrap();
    generate::prepare_questions(&vault, "demo", &planned.id)
        .await
        .unwrap();
    let topic_ids: Vec<&str> = plan
        .topics
        .iter()
        .map(|entry| entry.topic.id.as_str())
        .collect();
    assert_eq!(topic_ids, ["demo/1.1", "demo/1.2"]);
    assert_eq!(plan.open_questions.len(), 2);

    let question_for = |topic: &str| {
        plan.quiz
            .iter()
            .find(|question| question.topic_id == topic)
            .unwrap()
            .id
            .clone()
    };
    let open = BTreeMap::from([("demo/1.1".to_string(), "Моя відповідь".to_string())]);
    let quiz = BTreeMap::from([
        (question_for("demo/1.1"), vec![0]),
        (question_for("demo/1.2"), vec![1]),
    ]);
    let result = generate::finish_session(&vault, &plan.id, "demo", open, quiz)
        .await
        .unwrap();

    let scores: Vec<(u32, bool)> = result.topics.iter().map(|t| (t.score, t.skipped)).collect();
    assert_eq!(scores, [(94, false), (0, false)]);
    assert_eq!(result.overall_score, 47);
    assert_eq!(study::load(&vault).topics["demo/1.1"].level, 1);
    assert!(generate::list_unfinished(&vault, "demo").is_empty());

    let blank = generate::start_session(
        &vault,
        "demo",
        1,
        Some(vec!["demo/2.1".into()]),
        SessionMode::Sprint,
        Selection::Scheduled,
    )
    .await
    .unwrap();
    let result =
        generate::finish_session(&vault, &blank.id, "demo", BTreeMap::new(), BTreeMap::new())
            .await
            .unwrap();
    assert!(result.topics.iter().all(|topic| topic.skipped));
    assert!(!study::load(&vault).topics.contains_key("demo/2.1"));

    // Model economy: notes and grading at the smart tier, questions at the fast one, and no
    // call for a resumed preparation or a blank session.
    assert_eq!(
        calls(&root),
        [
            "fast session",
            "fast session",
            "smart grade",
            "smart knowledge",
            "smart knowledge",
            "smart knowledge",
        ]
    );

    std::fs::remove_dir_all(&root).ok();
}
