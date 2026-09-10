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

The digest leads with `## Головне` — the topic answered in one paragraph, the minimum that
would still pass, generated as `exam_answer` alongside the note. That is what a condensed
reading is for: `summary` says what the topic *is*, `## Головне` is what you would actually
write. Notes generated before the field existed have no such section and fall back to the
summary, so nothing needs regenerating; `--force` picks it up when you want it.

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
it is what the exam actually asks for. A half left blank is skipped, not failed: it is not
sent to the grader and not counted, and a topic with both halves blank gets no score at all,
so its schedule stays untouched — a student short on time must be able to read without
dragging the ladder down. That score moves the topic along a Leitner ladder in
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

Pace, selection and size persist to `localStorage` under one key (`src/lib/prefs.ts`), so the
results screen can offer **Продовжити навчання** and mean literally the same settings. Read
them back with `readSessionSettings`, never by reassembling the pieces. Hand-picked topics
are deliberately not persisted: they belong to one subject and one sitting.

**Model economy is a hard requirement.** Smart tier = knowledge generation + hints (rare,
cached on disk). Fast tier = quiz and session generation (frequent). Never generate knowledge
at the fast tier, never generate quizzes at the smart one. Which model each tier names is the
student's choice; which tier a job uses is not.

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

The same Rust code is reachable from the terminal via the `grind` CLI, a workspace member
beside the app, for bulk jobs too long to babysit in the GUI:

```bash
cd src-tauri && cargo run -p grind -- <subcommand>
```

It is a **separate crate on purpose**. As a second `[[bin]]` of the app package it broke
`tauri build --target universal-apple-darwin`: the universal build `lipo`s only the main
binary into `target/universal-apple-darwin/release/`, while the bundler copies every binary
the manifest declares, so it failed on a file that was never merged. One binary per package.

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
| `src-tauri/grind-cli/src/main.rs` | terminal CLI over the same functions (own crate) |
| `src-tauri/prompts/*.md` | prompt templates, `include_str!`-embedded |
| `src-tauri/schemas/*.json` | JSON schemas passed to `agy --json-schema` |

Prompts and schemas are embedded at compile time — the shipped app must not depend on files
next to the binary. The tray icon (`icons/tray.png`) is `include_bytes!`-embedded for the same
reason.

The app icon and the tray icon are the same graduation cap, drawn once by
`icons/make_icon.py`: a filled version on `--primary` for the Dock, and a black-on-transparent
template macOS recolours for the menu bar. Regenerate the platform set with
`bun run tauri icon icons/app-icon.png`.

## The `agy` CLI (Antigravity)

The only model access this project has. Always non-interactive, always structured:

```bash
agy -p "<prompt>" --model <id> --output-format json --json-schema <path> --print-timeout 10m
```

- stdout is a single JSON envelope: `{status, response, structured_output, usage, ...}`.
  **Read `structured_output`, not `response`** — `response` is prose and unreliable.
  Treat `status != "SUCCESS"` as a failure and retry once.
- `agy models` lists ids (`id<TAB>label`, one per line) — including Claude, Gemini and
  whatever else Antigravity exposes. Which model each tier resolves to is a setting, stored
  in `AppConfig::models` and pushed into `agy::configure` at startup and on save; the shipped
  defaults are `agy::DEFAULT_SMART` / `DEFAULT_FAST`. Never hardcode a model id at a call
  site — ask for a `ModelTier`.
- The tier a job runs at is **not** configurable, only the model behind it. Knowledge and
  hints are always `Smart`, quizzes and sessions always `Fast`.
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
dir, which is never permission-gated) → `<repo>/vault` (**debug builds only**) → app data dir
(packaged).

The repo probe is `cfg!(debug_assertions)`-gated because `CARGO_MANIFEST_DIR` is baked in at
compile time: in a packaged build it names a folder on the developer's machine, and merely
calling `is_dir()` on it is enough to raise a macOS folder-access prompt on first launch. For
the same reason `vault_info` does not touch the filesystem until `config.onboarded` is set —
before setup the app reads nothing, and the first system panel anyone sees is the folder
picker they asked for. That picker is also the macOS grant, so `choose_vault` / `create_vault`
are the only ways in.

`AppState` holds the vault behind a `Mutex` because `choose_vault` swaps it at runtime; read
it through `state.vault()`, never a stored copy. When file access fails, report *why* —
`paths::describe_vault` separates a denied macOS permission from a wrong folder, and the UI
depends on that distinction. Never let a permission error surface as an empty subject list.

A subject id is its syllabus filename — `syllabus/demo.md` is subject `demo`. A topic id is
`<subject>/<section-index>.<position>`, e.g. `demo/2.7`. Real subject ids and topic titles
belong in the vault, never in this repository: fixtures and examples here are invented.

Numbering in a syllabus may be nested — a section can list `1.` as a group heading with
`1.1.`…`1.8.` beneath it. `split_numbered` therefore parses the **whole** numeric prefix, and
`index` is the topic's position within its section, not the number printed next to it. A
group heading is not a topic; it is recorded as `Topic::group` and passed to the knowledge
prompt as context. Parsing only the first number collapses every nested item onto one id,
and because notes are addressed by `SS.TT-` prefix, their notes then silently overwrite each
other.

`vault/originals/` holds the source PDFs the syllabi came from — reference only, not read by
the app.

**Content language is Ukrainian.** Syllabi, knowledge notes, questions, explanations and
hints must all be Ukrainian. Prompts instructing the model are written in English but must
demand Ukrainian output — and that does **not** follow the interface language: the exam is in
Ukrainian whatever the student sets the chrome to.

**UI chrome is translated** (`react-intl`, Ukrainian default, plus Russian and English).
Never put a user-visible string in a component — add an id to
`src/i18n/messages/generate.py` and regenerate:

```bash
python3 src/i18n/messages/generate.py   # run from the repo root
```

The three catalogues come from one table so they cannot drift, and `en.ts` defines
`MessageId`, which types `useT()` — a typo in an id fails to compile. Anything holding a
label as data (`STAGES`, `PACE`, `DIFFICULTY_LABELS`) stores the id, not the text.

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

`bundle.targets` in `tauri.conf.json` must stay a cross-platform list (`app` + `nsis`). Tauri
filters it to the host platform, so one value serves both macOS and Windows CI legs.

**Do not add `dmg`.** Tauri's dmg bundler drives Finder over AppleScript purely to lay the
window out, and dies with `Not authorised to send Apple events to Finder (-1743)` on any
machine without Automation permission — CI included. The disk image is built in
`app-build.yml` with `hdiutil` instead: an `.app` plus an `/Applications` symlink, which
needs no permission and is the whole of what the image has to do.

macOS CI builds one `universal-apple-darwin` binary rather than a runner per architecture;
GitHub retired the `macos-13` Intel image.

```bash
bun run dev            # vite only
bun run tauri dev      # full app
bun run build          # tsc + vite build
cd src-tauri && cargo check
cd src-tauri && cargo run -p grind -- knowledge --subject demo
cd src-tauri && cargo run -p grind -- session --subject demo --size 2
cd src-tauri && cargo run -p grind -- session-finish --subject demo --session <id> --answers a.json
```
