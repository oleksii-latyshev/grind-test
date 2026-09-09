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

Study sessions come at three paces. **Full** works through the whole note and an essay-length
written answer, the way the oral exam is scored. **Balanced** keeps the same note but takes
the answer as a short list. **Sprint** also condenses the note down to the topic answered in
one paragraph, its key terms and its common traps. The app estimates how long each pace would take to get through what is
left.

Topics can be served by the review schedule, chosen by hand, or drawn **the way an exam paper
draws them** — one from each part of the syllabus, at random, leaning toward the weakest.

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
A packaged macOS app does not inherit the shell `PATH`, so it falls back to
`~/.local/bin/agy` and the Homebrew prefixes.

To build the installable app:

```bash
bun run tauri build   # → src-tauri/target/release/bundle/macos/grind-test.app
```

Bundle targets are `app` + `dmg` + `nsis`; Tauri builds only the ones that apply to the host,
so macOS produces the `.app` and the `.dmg` and Windows the installer. The dmg bundler drives
Finder over AppleScript and so needs a GUI session — on a headless shell (ssh, a container)
pass `--bundles app` instead.

A debug build reads the `vault/` beside the checkout; a release build does not, because that
path is baked in at compile time and would point at the build machine.

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

`vault/` is gitignored — it holds personal exam material and results.

On first launch the app asks where to keep it — point at an existing folder, create a new
vault, or keep it inside the app's own data directory. Nothing on disk is read before that
answer, so macOS never raises a folder prompt for somewhere you did not name.

After setup it resolves the vault in this order: `$GRIND_VAULT`, then the folder you picked,
then a `vault/` directory next to the repository (debug builds only), then the app data dir.
The choice is stored in the OS config directory, which is never gated behind file-access
permission, so the app can always remember where the vault is even when it cannot currently
read it.

Use **Обрати теку сховища** on the subjects screen to point the app somewhere else. On macOS
this is also the way back from a declined folder-access prompt: choosing a folder in the
system panel is an explicit grant.

Architecture details live in [CLAUDE.md](CLAUDE.md).

## CI

`.github/workflows/build.yml` runs on every push to `main`, on pull requests, and on demand.
A fast `typecheck` job (`tsc` + `vite build`, no Rust) gates a build matrix that runs the Rust
tests and then packages the app for macOS arm64, macOS x64 and Windows x64.

Download a build from the run's **Artifacts** section:

| Artifact | Contents |
|---|---|
| `grind-test-macos` | universal `.dmg`, plus the `.app` zipped with `ditto` so the bundle stays executable |
| `grind-test-windows-x64` | NSIS `*-setup.exe` |

The macOS leg builds `universal-apple-darwin`, so one download covers Apple Silicon and
Intel.

Builds are unsigned, so macOS shows an unidentified-developer warning on a downloaded build
(right-click → Open, once). A CI build has no repository beside it, so it starts with an empty
vault — point it at yours with **Обрати теку сховища**.

For public distribution the usual next step is a tag-triggered job that uploads the same
bundles to a GitHub Release; this workflow deliberately stops at artifacts.
