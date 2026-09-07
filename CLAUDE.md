# grind-test — exam prep quiz generator

Desktop app (macOS + Windows) that turns a university exam syllabus into a personal
knowledge base and generates practice quizzes from it, tracking what the user gets wrong.

## Core pipeline

```
vault/syllabus/<subject>.md   (INPUT, hand-made: list of exam topics per subject)
        │  smart model  (gemini-3.1-pro-high / claude-opus-4-6-thinking)
        ▼
vault/knowledge/<subject>/<NNN>-<slug>.md   (one file per topic, generated once, reused)
        │  fast model  (gemini-3.8-flash-medium)  +  vault/progress/mastery.json
        ▼
vault/quizzes/<subject>/<quiz-id>.json      (N questions, grounded in knowledge notes)
        │  user takes the quiz in the app
        ▼
vault/progress/attempts/<attempt-id>.json  →  vault/progress/mastery.json
        └─ feeds back into topic weighting for the next quiz
```

Hints during a quiz are a separate on-demand call to the **smart** model, grounded in the
knowledge note for that question's topic. A hint must never contain the answer.

## Study mode

The quiz pipeline tests what you know; study mode is how you learn it in the first place, and
it is the primary way into the app. One session covers 2-6 topics:

```
read the notes  →  answer one open question per topic  →  mini-quiz  →  graded, rescheduled
                   (exam format: 3 broad questions)      (4 per topic)
```

Cost per session is fixed at **two model calls**: one fast call producing the open questions
(with an `expected_points` rubric) and the whole mini-quiz together, one smart call grading
every written answer at once. Never grade answers one call at a time.

A topic's session score is `0.6 × open + 0.4 × quiz` — the written half weighs more because
it is what the exam actually asks for. That score moves the topic along a Leitner ladder in
`vault/progress/study.json`: pass (≥80) climbs a level, ≥60 holds, below 60 drops one, and
the new level sets the review date (1 → 2 → 4 → 7 → 14 → 30 days). `study::plan_session`
serves overdue reviews first, then unseen topics in syllabus order.

Sessions persist to `vault/progress/sessions/` on start, so quitting mid-session loses
nothing, and grading reads the plan back from disk rather than trusting the client.

**Model economy is a hard requirement.** Smart model = knowledge generation + hints (rare,
cached on disk). Fast model = quiz generation (frequent). Never generate knowledge with the
fast model, never generate quizzes with the smart model.

## Stack

- **Tauri v2** (Rust backend) + **React 19** + **Vite 8** + **TypeScript** + **Bun**
- **Tailwind v4** (`@tailwindcss/vite`, config-less, tokens live in `src/App.css`)
- **shadcn/ui**, style `base-maia`, base color `zinc`, icons `lucide-react`, components in
  `src/components/ui/`. An MCP server (`shadcn`) is configured in `.mcp.json` — use its tools
  (`search_items_in_registries`, `get_add_command_for_items`) to pull missing components
  instead of hand-writing them.
- Package manager is **bun** (`bun.lock`). Use `bun add`, `bun run`, never npm/yarn.

## Rust owns the vault

All vault reading/writing, syllabus parsing and `agy` invocation lives in Rust
(`src-tauri/src/`) and is exposed to React as Tauri commands. **Do not reimplement vault
logic in TypeScript** — the frontend only consumes typed JSON from `invoke()`.

The same Rust code is reachable from the terminal via a second binary for bulk jobs that are
too long to babysit in the GUI:

```bash
cd src-tauri && cargo run --bin grind -- <subcommand>
```

Layout:

| Path | Responsibility |
|---|---|
| `src-tauri/src/vault/paths.rs` | resolve vault root, subject/topic paths, slugs |
| `src-tauri/src/vault/syllabus.rs` | parse `vault/syllabus/*.md` → subjects/sections/topics |
| `src-tauri/src/vault/knowledge.rs` | read/write knowledge notes + frontmatter |
| `src-tauri/src/vault/quiz.rs` | quiz model, read/write `vault/quizzes/` |
| `src-tauri/src/vault/progress.rs` | attempts, mastery aggregation, topic weighting |
| `src-tauri/src/vault/study.rs` | study state, session planning, review scheduling |
| `src-tauri/src/agy.rs` | spawn the `agy` CLI, parse its JSON envelope |
| `src-tauri/src/generate.rs` | prompt assembly + the three generation jobs |
| `src-tauri/src/commands.rs` | `#[tauri::command]` surface |
| `src-tauri/src/bin/grind.rs` | terminal CLI over the same functions |
| `src-tauri/prompts/*.md` | prompt templates, `include_str!`-embedded |
| `src-tauri/schemas/*.json` | JSON schemas passed to `agy --json-schema` |

Prompts and schemas are embedded at compile time — the shipped app must not depend on files
next to the binary.

## The `agy` CLI (Antigravity)

The only model access this project has. Always non-interactive, always structured:

```bash
agy -p "<prompt>" --model <id> --output-format json --json-schema <path> --print-timeout 10m
```

- stdout is a single JSON envelope: `{status, response, structured_output, usage, ...}`.
  **Read `structured_output`, not `response`** — `response` is prose and unreliable.
  Treat `status != "SUCCESS"` as a failure and retry once.
- `agy models` lists ids. Current picks: smart `gemini-3.1-pro-high`, fast
  `gemini-3.8-flash-medium`, hints `gemini-3.1-pro-high`. Keep these in one place
  (`src-tauri/src/agy.rs`), never hardcode a model id at a call site.
- Every call has a **~13k token system-prompt floor**, so batch related work into one call
  where quality allows (e.g. all questions for a quiz = one call) but keep one call per
  knowledge topic (quality matters more there).
- `agy` is an *agent* with file tools. We do not want it touching the repo: pass all context
  inline in the prompt, and spawn it with cwd set to a scratch dir so it does not pick up
  `CLAUDE.md`/`AGENTS.md` and burn tokens.
- On macOS a bundled `.app` does not inherit the shell `PATH`. Resolve the binary explicitly:
  `$GRIND_AGY_BIN` → `PATH` → `~/.local/bin/agy` → `/opt/homebrew/bin/agy`.
- Never pass `--dangerously-skip-permissions`.

## Vault

`vault/` is **gitignored** — it holds the user's personal exam material and results. Never
commit it, never assume a file in it exists, always degrade gracefully when it is empty.

Vault root resolution: `$GRIND_VAULT` → path stored in app config → `<repo>/vault` (dev) →
app data dir (packaged).

Subjects are the syllabus filenames: `f2`, `f3`, `f7` (242 topics total). A topic id is
`<subject>/<section-index>.<topic-index>`, e.g. `f3/2.7`.

`vault/originals/` holds the source PDFs the syllabi came from — reference only, not read by
the app.

**Content language is Ukrainian.** Syllabi, knowledge notes, questions, explanations and
hints must all be Ukrainian. UI chrome is Ukrainian too. Prompts instructing the model are
written in English but must demand Ukrainian output.

## Generation must be resumable

242 topics × one smart call is slow and expensive. Knowledge generation:
skips topics that already have a note (unless `--force`), runs a bounded number of calls
concurrently (default 4), writes each note the moment it lands, and reports progress via
Tauri events (`generation://progress`). A crash or quit must never lose completed work.

## Conventions

- Ukrainian text is the payload; code, comments, identifiers, commit messages and every
  documentation file (README.md, this file) are English.
- Slugs: transliterate Cyrillic → ASCII, lowercase, `-`-separated, truncated to ~60 chars.
  Filenames are prefixed with a zero-padded index so directory order matches syllabus order.
- Timestamps are RFC3339 UTC.
- Prefer `serde` structs over `serde_json::Value` for anything crossing the Rust↔TS boundary,
  and keep the mirrored TS types in `src/lib/types.ts` in sync by hand.

## Commands

```bash
bun run dev            # vite only
bun run tauri dev      # full app
bun run build          # tsc + vite build
cd src-tauri && cargo check
cd src-tauri && cargo run --bin grind -- knowledge --subject f3
cd src-tauri && cargo run --bin grind -- session --subject f7 --size 2
cd src-tauri && cargo run --bin grind -- session-finish --subject f7 --session <id> --answers a.json
```
