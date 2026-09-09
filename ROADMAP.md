# Roadmap

Each numbered item lands as its own commit. Items 1–12 are the requested list; the
"Ideas" section at the end is what this codebase is one step away from and would gain the
most from.

## 1. Fix CI, and ship a `.dmg`

**Why CI was red.** `.gitignore` line 26 was the bare pattern `vault`. Git patterns without
a slash match a path segment at *any* depth, so it excluded the intended `vault/` — and also
`src-tauri/src/vault/`, the module that owns syllabus parsing, knowledge notes, quizzes,
progress and study state. Nine Rust files were never committed. Every macOS and Windows leg
failed identically with `error[E0583]: file not found for module 'vault'`; only the
Rust-free `typecheck` job passed, which is why the breakage looked intermittent.

Fix: anchor the pattern to `/vault/`, commit the module, and add a CI guard so a missing
source file fails in seconds instead of after a full toolchain install.

**Why `.app` and not `.dmg`.** The old note in `CLAUDE.md` was right about the symptom and
wrong about the scope: Tauri's dmg bundler drives Finder over AppleScript to lay out the
window, which fails in a shell with no GUI session (ssh, a container). GitHub's macOS
runners *do* have a session, so the constraint only ever applied to local headless builds.
`.dmg` is what users expect to download, so it goes back in — with the `.app` still produced
for anyone who wants the raw bundle.

Also in this commit: `macos-13` was retired by GitHub, so the Intel leg is replaced by a
single `universal-apple-darwin` build on `macos-latest` that covers both architectures, and
`actions/checkout` moves off the deprecated Node 20 runtime.

## 2. Show the notes immediately, generate the questions in the background

Starting a session currently blocks on one fast-model call before anything renders, even
though the first thing shown — the knowledge note — is already on disk and needs no model
at all.

Split `start_study_session` in two:

| command | cost | when |
|---|---|---|
| `plan_study_session` | disk only | on click; returns topics + notes, persists the plan |
| `prepare_session_questions` | one fast call | fired in the background as reading starts |

The reader opens instantly on note 1. The generation call runs while the student reads, and
only the last "До питань" button can ever wait — by which point, in every mode, the call has
had a whole note's worth of reading time to finish. A failure surfaces there with a retry
rather than as a dead end before the session began.

## 3. First-run setup, and no unexplained Desktop prompt

`resolve_vault_root` probes `<CARGO_MANIFEST_DIR>/../vault` — a path baked in at compile
time. In a packaged build that path points at the developer's Desktop, so `is_dir()` on it
triggers a macOS folder-access prompt for Desktop on first launch, for a folder the app has
no business reading.

The dev fallback becomes `cfg!(debug_assertions)`-only. A packaged build with no configured
vault does not touch the filesystem at all: it shows a setup screen where the student either
points at an existing vault or creates a new one, which is the only moment a system panel
appears — and picking a folder there is an explicit, scoped grant.

Config gains `onboarded`, so the wizard is a one-time event rather than something that
returns whenever the vault happens to be empty.

## 4. The main idea as one paragraph

The digest currently opens with `summary` — 2–3 sentences on *what the topic is and why it
matters*. That orients, but it is not what the exam asks for.

The knowledge prompt gains `exam_answer`: one dense paragraph that is the minimum answer a
student could write on this topic and still pass — the definition, the mechanism, the
distinction that matters. It is stored as `## Головне` in every note and leads the digest.
Notes generated before this change fall back to the summary lead, so nothing needs
regenerating.

## 5. Remember the session settings, and continue without going back

Pace, selection and size reset on every visit. They move to `localStorage` behind a small
`usePreference` hook.

The end-of-session screen gains **Продовжити** next to **Далі**: it starts the next session
with the same settings, so a study run is a loop rather than a round trip through the menu.

## 6. Pomodoro

A timer in the session header: 25 minutes of work, 5 of rest, a long break every fourth
round, all configurable and persisted. It notifies at each transition and does not interrupt
what is on screen — the point is to stop a three-hour cram, not to gate the UI.

## 7. Choosing the models

`ModelTier::model_id()` hardcodes two Gemini ids. It becomes a runtime lookup over settings
persisted in the app config, defaulting to today's values. A settings screen lists what
`agy models` reports — which is where Claude, Gemini and the rest already coexist — and lets
each tier be set independently.

The invariant in `CLAUDE.md` still holds and is now enforced rather than assumed: knowledge
and hints take the smart tier, quizzes and sessions the fast one. What changes is *which*
model each tier names, not which tier a job may use.

## 8. Tray icon

A graduation cap, drawn once and rendered into every size the platforms want — the app icon,
and a template tray icon that follows the macOS menu-bar theme. The tray menu shows/hides the
window and quits.

## 9. Guide

A "Як це працює" screen: the pipeline, what each pace costs, what the ladder does with a
score. Shown once after setup with a visible skip, and reachable from the header afterwards.

## 10. Releases

A tag-triggered workflow (`v*`) that runs the same matrix as CI and publishes the `.dmg`,
`.app` zip and NSIS installer to a GitHub Release, with notes generated from the commits
since the previous tag.

## 11. Interface language

`react-intl` with Ukrainian, Russian and English catalogues. Ukrainian stays the default and
stays the language of the *content* — the model is still instructed to write notes,
questions and hints in Ukrainian regardless of the interface language, because that is the
language of the exam. This item moves chrome only.

## 12. README

Cut to what someone deciding whether to run it needs, opening with a screenshot of the
subjects screen — rendered from neutral placeholder subjects in English, so nothing personal
is published.

---

## Ideas worth doing next

Ordered by what this codebase would gain per unit of work.

**Mock exam mode.** Everything needed already exists: `Selection::Spread` draws one topic
per part of the syllabus the way a ticket does, `grade_open` scores written answers against
a rubric in a single call. A mock exam is those two with the notes taken away and a clock
added — three broad questions, no reading step, graded as a whole. It is the only mode that
measures what the actual exam measures, and it is perhaps a day of work.

**Recall before reading.** Sessions read, then answer. Reversing that for topics already on
the ladder — ask first, let the student fail, *then* show the note — is the single
best-evidenced change available here, and it costs nothing: the questions are generated
either way.

**Confidence on each answer.** One tap per question. It separates "does not know" from
"wrong and certain", which are different problems and the second is what loses exam marks.
It also sharpens the ladder: a confidently wrong answer should drop further than a blank.

**An error journal.** Every `missed` rubric point and every wrong quiz option is already
computed and then thrown away after the results screen. Collecting them per topic gives a
single reviewable list of exactly what this student keeps getting wrong, and gives the
session generator something concrete to aim the next questions at.

**Exam date and a plan backwards from it.** The app already estimates hours remaining per
pace. Given a date it can say whether the current rate arrives in time, and how many topics
a day would.

**Flashcards from the key terms.** `## Ключові терміни` is in every note and is exactly a
term/definition deck. Two-sided cards, no model call, no new data.

**Interleaving across subjects.** Sessions are scoped to one subject. Drawing from all three
in one sitting matches how the exam period actually works and is better for retention than
blocking by subject.
