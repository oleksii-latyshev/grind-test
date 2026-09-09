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

Starting a session is split so nobody waits for material that is already on disk.
`plan_session` picks the topics and loads the notes — disk only, returns at once — and
`prepare_questions` is the fast call, fired by the client as the first note opens and
finishing while it is read. An empty `quiz` is what marks a plan as not yet prepared;
`prepare_questions` is idempotent, so a resumed session never pays twice.

`SessionMode` picks the trade-off, cutting the two costly halves — reading the note and
writing the answer — in that order:

| | reading | answer | quiz/topic | ~min/topic |
|---|---|---|---|---|
| `Full` | whole note | several paragraphs | 4 | 20 |
| `Balanced` | whole note | short list | 4 | 13 |
| `Sprint` | `knowledge::digest` | short list | 3 | 8 |

Cutting the reading is what makes a fast session forgettable — a digest carries terms and
traps but not the mechanism, and you cannot recall what you were never shown. So `Balanced`
gives up the essay and keeps the note; only `Sprint` gives up the note. The digest needs no
model call: the generator already writes those sections into every note.

`Selection` decides which topics when they are not listed explicitly. `Scheduled` follows the
ladder (or, in a sprint, `plan_sprint` — unseen topics in syllabus order, since a sprint is
about coverage). `Spread` cuts the candidates into as many equal slices as there are topics
to pick and takes one from each, weighted toward the weakest: the exam draws its questions
from across the course, and always walking the syllabus front to back never rehearses that.

Questions are always generated from exactly what the student was shown, so a sprint can never
be tested on material its digest omitted. The grading prompt is told which mode produced the
answer — without that, a deliberately terse sprint answer gets marked down for terseness and
drags the whole ladder with it.

A topic's session score is `0.6 × open + 0.4 × quiz` — the written half weighs more because
it is what the exam actually asks for. That score moves the topic along a Leitner ladder in
`vault/progress/study.json`: pass (≥80) climbs a level, ≥60 holds, below 60 drops one, and
the new level sets the review date (1 → 2 → 4 → 7 → 14 → 30 days). `study::plan_session`
serves overdue reviews first, then unseen topics in syllabus order.

Sessions persist to `vault/progress/sessions/` on start, so quitting mid-session loses
nothing, and grading reads the plan back from disk rather than trusting the client. An
ungraded session stays resumable (`unfinished_sessions` / `resume_study_session`) and the
written answers are drafted to `localStorage` while typing, so an interrupted session costs
neither the generation call nor the typing.

`plan_session` is the default, but an explicit `topic_ids` list overrides it — the student
knows better than the ladder which topics the exam is about to ask for. Either way a session
is capped at `MAX_SESSION_TOPICS`.

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
| `src-tauri/src/config.rs` | persisted settings (the chosen vault folder) |
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

Vault root resolution: `$GRIND_VAULT` → the folder the user picked (stored in the OS config
dir, which is never permission-gated) → `<repo>/vault` (dev) → app data dir (packaged).

`AppState` holds the vault behind a `Mutex` because `choose_vault` swaps it at runtime; read
it through `state.vault()`, never a stored copy. When file access fails, report *why* —
`paths::describe_vault` separates a denied macOS permission from a wrong folder, and the UI
depends on that distinction. Never let a permission error surface as an empty subject list.

Subjects are the syllabus filenames: `f2`, `f3`, `f7` (252 topics total). A topic id is
`<subject>/<section-index>.<position>`, e.g. `f3/2.7`.

Numbering in a syllabus may be nested — f7's section 1.7 lists `1.` as a group heading with
`1.1.`…`1.8.` beneath it. `split_numbered` therefore parses the **whole** numeric prefix, and
`index` is the topic's position within its section, not the number printed next to it. A
group heading is not a topic; it is recorded as `Topic::group` and passed to the knowledge
prompt as context. Parsing only the first number collapses every nested item onto one id,
and because notes are addressed by `SS.TT-` prefix, their notes then silently overwrite each
other.

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
- In React event handlers, read `event.target.value` into a local before calling `setState`.
  A functional updater runs during the render phase, by which point React has nulled
  `event.currentTarget` — reading it in there throws mid-render and unmounts the whole tree.
- Keep the ladder thresholds in `src/lib/study.ts` (`stageOf`), never inline in a component.

## Commands

`bundle.targets` in `tauri.conf.json` must stay a cross-platform list (`app` + `dmg` +
`nsis`). Tauri filters it to the host platform, so one value serves both macOS and Windows CI
legs. The `dmg` bundler drives Finder over AppleScript, so it needs a GUI session: it works
on a normal desktop and on GitHub's macOS runners, and fails over ssh or in a container —
build `--bundles app` there.

macOS CI builds one `universal-apple-darwin` binary rather than a runner per architecture;
GitHub retired the `macos-13` Intel image.

```bash
bun run dev            # vite only
bun run tauri dev      # full app
bun run build          # tsc + vite build
cd src-tauri && cargo check
cd src-tauri && cargo run --bin grind -- knowledge --subject f3
cd src-tauri && cargo run --bin grind -- session --subject f7 --size 2
cd src-tauri && cargo run --bin grind -- session-finish --subject f7 --session <id> --answers a.json
```
