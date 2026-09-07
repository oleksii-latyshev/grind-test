# grind-test

A desktop app (macOS + Windows) for exam prep: it turns a list of exam topics into a
personal knowledge base, generates practice quizzes from it, and remembers where you keep
going wrong.

## How it works

```
vault/syllabus/<subject>.md      ← your topic list (one file per subject)
        │  smart model
        ▼
vault/knowledge/<subject>/*.md   ← one study note per topic, generated once
        │  fast model + your answer history
        ▼
vault/quizzes/<subject>/*.json   ← a quiz
        │  you take it
        ▼
vault/progress/                  ← attempts + a mastery table that weights
                                   topic selection for the next quiz
```

Asking for a hint mid-quiz is a separate call to the smart model, grounded in that topic's
study note; it never names the correct option.

All generation goes through the [Antigravity](https://antigravity.google) CLI (`agy`).

## Language

Code, comments, identifiers and documentation are English. Everything the student reads —
the UI, study notes, questions, explanations and hints — is Ukrainian, because that is the
language of the exam.

## Running it

```bash
bun install
bun run tauri dev
```

Requires `agy` on your `PATH` (or its path in the `GRIND_AGY_BIN` environment variable).

## CLI for bulk jobs

Filling the knowledge base for 250+ topics is easier from a terminal than from the GUI:

```bash
cd src-tauri
cargo run --bin grind -- status
cargo run --bin grind -- topics --subject f7 --missing
cargo run --bin grind -- knowledge --subject f7 --concurrency 4
cargo run --bin grind -- quiz --subject f7 --count 10
```

Knowledge generation is resumable: topics that already have a note are skipped, so an
interrupted run can simply be started again.

## The vault

`vault/` is gitignored — it holds personal exam material and results. It is located via
`$GRIND_VAULT`, then a `vault/` directory next to the repository, then the app data dir.

Architecture details live in [CLAUDE.md](CLAUDE.md).
