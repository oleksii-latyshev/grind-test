# grind-test

<p align="center">
  <img src="docs/screenshot.png" alt="The subjects screen: four exam subjects, each showing how many topics already have a study note, how many quizzes exist, and the running accuracy." width="900">
</p>

<p align="center">
  A desktop app for exam prep. Give it your list of exam topics; it writes you a study note
  for each one, walks you through them, and remembers what you keep getting wrong.
</p>

---

## How it works

```
vault/syllabus/<subject>.md      ← your topic list (one file per subject)
        │  smart model
        ▼
vault/knowledge/<subject>/*.md   ← one study note per topic, generated once
        │  fast model + your answer history
        ▼
a study session                  ← read, answer in writing, mini-quiz
        │  graded by the smart model
        ▼
vault/progress/                  ← a review schedule that decides what you see next
```

A **session** is the core of it: two to six topics, read the note, write an answer to one
broad exam-style question per topic, then a mini-quiz. The written half is graded against a
rubric — what you covered, what you missed — and counts for 60% of the topic's score. That
score moves the topic along a Leitner ladder: 1 → 2 → 4 → 7 → 14 → 30 days.

Three paces trade depth for coverage. **Full** takes the whole note and an essay-length
answer. **Balanced** keeps the note and takes the answer as bullet points. **Sprint** also
condenses the note to the topic answered in one paragraph, its key terms and its usual traps.
The answer is always cut before the reading — you cannot recall a note you were never shown.

Topics come from the review schedule, from you, or drawn **the way an exam paper draws
them**: one from each part of the syllabus, leaning toward your weakest.

## Running it

```bash
bun install
bun run tauri dev
```

Requires the [Antigravity](https://antigravity.google) CLI (`agy`) on your `PATH`, or its
path in `GRIND_AGY_BIN`. All generation goes through it, split across two tiers — a smart
model for notes, grading and hints, a fast one for quizzes and session questions. **Моделі**
in the header picks which model each tier uses, from whatever `agy models` lists (Gemini,
Claude and the rest). Which tier a job runs at is fixed: a study note is worth paying for, a
quiz is not.

To build an installable app:

```bash
bun run tauri build
```

macOS gets a universal `.dmg` and the `.app`; Windows gets an NSIS installer. The dmg
bundler drives Finder over AppleScript, so on a headless shell pass `--bundles app`.

## The vault

Everything the app knows lives in one folder you choose: syllabi in, notes and results out,
as ordinary markdown and JSON. On first launch it asks where — point at an existing folder,
create a new one, or keep it inside the app's own data directory. Nothing on disk is read
before you answer, so macOS never raises a folder prompt for somewhere you did not name.

A syllabus is a markdown file: a title, sections under `##`, a numbered list of topics under
each. The filename becomes the subject id. Creating a vault leaves a sample in it.

`vault/` at the repo root is gitignored — it holds personal exam material.

## CLI for bulk jobs

Filling the knowledge base for 250+ topics is easier from a terminal than from the GUI, and
it is resumable — topics that already have a note are skipped, so an interrupted run just
gets started again.

```bash
cd src-tauri
cargo run --bin grind -- status
cargo run --bin grind -- topics --subject cs1 --missing
cargo run --bin grind -- knowledge --subject cs1 --concurrency 4
```

## Language

The interface speaks Ukrainian, Russian or English — switch it in the header. The content
does not follow: study notes, questions, explanations and hints are always Ukrainian, because
that is the language of the exam.

## CI and releases

Every push runs a typecheck, the Rust tests, and a full package for macOS and Windows.
Pushing a `v*` tag runs the same matrix and publishes the bundles to a GitHub Release.

```bash
# bump `version` in src-tauri/tauri.conf.json and package.json first
git tag v0.2.0 && git push origin v0.2.0
```

Builds are unsigned, so macOS refuses a downloaded one on first open — right-click → **Open**,
once.

---

Architecture and conventions live in [CLAUDE.md](CLAUDE.md); what is planned and why is in
[ROADMAP.md](ROADMAP.md).
