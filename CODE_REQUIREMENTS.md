# CODE_REQUIREMENTS.md

How code in this repository is written. *What* the app does and *why* it is built this way is in
[CLAUDE.md](CLAUDE.md). The bar: a reviewer signs off without a follow-up conversation.

Everything in the repository is English. Only user-facing strings are translated (§5), and study
content is Ukrainian (CLAUDE.md).

## 1. Where code lives

```
src/
  App.tsx               the view switch and the header
  components/           shared by more than one feature; ui/ is vendored shadcn, not linted
  lib/                  shared and DOM-free: api, types, preferences, utils
  features/<feature>/   one user-facing area: guide, pomodoro, quiz, settings, study, subjects, vault
    components/         that feature's components
    lib/                that feature's logic, tests beside it
  i18n/                 react-intl provider; messages/ is generated
e2e/                    Playwright against Vite, command layer faked
src-tauri/src/          the vault, agy, generation, the command surface
src-tauri/tests/        the pipeline against a temp vault and a fake agy
```

- Code starts in the feature that uses it and moves to `components/` or `lib/` when a second
  feature needs it — not before.
- Features depend one way: `subjects → study, quiz, vault` and `guide → study`; the rest are
  leaves. No cycles, and `components/` / `lib/` never import a feature. When two features need
  each other, the shared part belongs in `components/` or `lib/`.
- Import through `@/…`; only files in the same folder import each other relatively. No barrels.
- **Files stay under 300 lines**, enforced by Biome (`noExcessiveLinesPerFile`). A component that
  outgrows it becomes a folder named after it (`features/study/components/StudySession/`), one
  file per part. Split by screen or by job (`generate/session.rs`), never by kind (`hooks/`,
  `helpers/`). An exception — a generated file, a data table — is an override in `biome.json`,
  never a silent suppression. Rust follows the same limit; nothing enforces it there yet.
- **Rust owns the vault.** A rule that decides something — scoring, scheduling, what a session
  contains — lives in Rust and the UI renders its result. A rule the UI needs without a round
  trip (`stageOf`) lives in `src/lib`, once, and names its Rust counterpart.

## 2. Comments

Comment the *why* the code cannot say: a platform workaround, a constraint from `agy` or macOS, a
choice that looks wrong and is not. Never restate the code, narrate history, or leave a `TODO`.
Doc comments only where the signature does not explain itself. Every `biome-ignore` states its
reason.

## 3. Naming

- `camelCase` values and functions, `PascalCase` types and components,
  `SCREAMING_SNAKE_CASE` module constants. Rust follows `rustfmt`.
- Fields that cross `invoke()` stay `snake_case` on both sides; do not rename them in TS.
- Units in names: `durationMs`, `dueAt`, `MINUTES_PER_TOPIC`.
- Booleans read as assertions: `ready`, `hasNotes`, `canStart` — never `flag` or `status`.
- `handleX` implements, `onX` is the prop.
- Components `PascalCase.tsx`; other modules lowercase, named for what they do. No `utils.ts`
  dumping ground.

## 4. Types and errors

- `strict`; no `any`, no non-null `!`, no `as` to silence the compiler. Narrow or validate.
- Whatever is read back — `localStorage`, JSON, an older build's data — is `unknown` until a
  guard accepts it (`isSessionSettings`).
- States are unions, not bags of flags; `View` in `App.tsx` is the pattern.
- `src/lib/types.ts` mirrors the serde structs by hand. A change to a struct that crosses
  `invoke()` updates `types.ts` and the E2E fixtures in the same commit.
- Never swallow an error. An empty `catch` is only for failures that are harmless by design
  (storage unavailable), and says so. Tauri rejects with a string: show it through
  `errorMessage`.
- Rust: `anyhow` with `context` naming what was being done. When the UI must branch on a kind
  of failure, Rust tells the kinds apart (`describe_vault`); TS never matches on a message.

## 5. React

- Function components, props typed inline or as a local `Props`. No classes except
  `ErrorBoundary`.
- No effect for derived state — compute during render. Effects sync with the outside
  (`invoke`, storage, timers, the DOM) and clean up after themselves.
- No `useMemo` / `useCallback` by reflex; only for a real cost or a dependency that must stay
  stable.
- One level of ternary in JSX. Past that, return early or extract a component.
- Read `event.target.value` into a local before `setState` (CLAUDE.md has the crash).
- Every user-visible string goes through `useT()`, with its id added to `generate.py`. Data that
  holds a label holds the id.
- Colour and radius come from the tokens in `App.css`.
- Every `localStorage` key is unique app-wide; check the existing ones before adding one.

## 6. Shape

- Guard clauses over nesting.
- More than ~4 parameters: an options object in TS, a struct in Rust.
- Export what is used, nothing speculative.
- No dependency for what the platform or an installed package already does.
- Model tiers, not model ids, at call sites (CLAUDE.md).

## 7. Tests

| What | Where | Command |
|---|---|---|
| Pure TS logic | `*.test.ts` beside it in a `lib/`, no DOM | `bun test` |
| Rust logic | `#[cfg(test)] mod tests` beside the code | `bun run test:rust` |
| Syllabus → notes → session → grade | `src-tauri/tests/pipeline.rs` | `bun run test:rust` |
| User flows | `e2e/*.e2e.ts` | `bun run test:e2e` |

- To test logic, move it out of the component into a `lib/` or Rust. Needing a DOM for a unit
  test means the logic is in the wrong place.
- No mocks of code we own. Two seams are faked, both at the edge: `agy` (through
  `GRIND_AGY_BIN`) and, in E2E only, `invoke()`, whose fixtures are typed against `types.ts`.
- E2E covers flows, not details. Select by role and visible English text (the fixture sets the
  locale), never by class name. A new command a flow needs gets a default handler in
  `e2e/fixtures.ts`; an unhandled one fails the test.
- Every bug fix starts with a test that reproduces it.
- No snapshot tests of rendered output.

## 8. Before pushing

```bash
bun run check       # biome lint + format; check:fix applies
bun run typecheck
bun test
bun run test:rust
bun run test:e2e
```

CI runs all of them. Rust is formatted with `cargo fmt`.
