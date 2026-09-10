# CODE_REQUIREMENTS.md

How this codebase is organized and how to write code that fits in. For *what* the product does and
*why* it is built this way, see [AGENTS.md](AGENTS.md).

The standard is: **code a senior engineer would sign off on without a follow-up conversation.**

Everything in the repository — code, comments, docs, commit messages, issue titles — is in English.
Only user-facing strings are translated, and they live in locale files (§10).

---

## 1. Layout

Repository-level layout (`apps/`, `packages/`, `crates/`) is in `AGENTS.md` §4. This is the inside
of `apps/web`:

```
apps/web/src/
  core/           App-specific building blocks. No dependency on features.
    playback/     rAF clock binding, interpolation reads, transport state machine.
    renderer/     Canvas setup, layer compositing, resize, device-pixel handling.
                  Knows nothing about CS2 — it draws what it is told.
    parsing/      Parse worker lifecycle, progress state machine, cancellation.
    shortcuts/    Keyboard registry and scope handling.
    motion/       The one orchestrated moment, as timings two slices read.
    settings/     UI preferences: playback rules, plate options, locale, palette.
                  There is no theme provider — dark is unconditional (AGENTS.md §20).
  features/       User-facing slices. May use core and other features.
    library/      Open/drop a demo, parse progress, error screens, cached demo list.
    review/       The stage: the four cards, the timeline block, and what they compose.
    timeline/     The match ribbon and the round timeline: bands, density, economy, scrubber.
    radar/        Map rendering: layers, player tokens, grenades, sound circles.
    filters/      Filter builder UI and highlight extraction controls.
    controls/     Playback transport: play/pause, speed, tick stepping.
    settings/     Locale, colour-blind mode, debug/calibration overlay.
  shared/         App-agnostic code any layer may use — nothing here knows about CS2.
    hooks/          Generic React hooks (reduced motion, media query, resize observer).
    lib/            `cn()` and similar tiny utilities.
```

Every `core/` and `features/` folder groups its own `components/`, `helpers/`, `hooks/`,
`constants/`, `__tests__/` as needed, plus an `index.ts` barrel.

### Package names

Workspace packages are named `@disa/<folder>` — `@disa/demo-core`, `@disa/map-data`, `@disa/ui`.
The folder name and the package name always match.

- **Always import a package by its name, never by filesystem path.** `import { … } from
  '@disa/demo-core'`, never `'../../packages/demo-core/src/…'`. Turborepo builds its task graph and
  cache keys from the dependencies declared in each `package.json`; a path import is invisible to
  it, so `demo-core` will not be rebuilt before `web` and the cache will serve stale output. The
  same applies to TypeScript project references.
- Adding a cross-package import means adding the dependency to that package's `package.json`. If
  that feels wrong, the code is in the wrong layer.
- The scope is `@disa`, not `@disalytics`, so it stays reusable across other projects, and it never
  gets published to npm by accident — `"private": true` on every package that is not intended for
  release.
- Path aliases (`@/core/playback`, `@/features/timeline`) are for *inside* `apps/web` only. They
  never cross a package boundary.

---

### `packages/*` versus `apps/web/src/core/*`

One question decides it: **would a second consumer need this?** `apps/api`, `apps/landing`, or a
Tauri shell.

- Yes → `packages/*`. Parsing, storage, the event schema, map data, locale resources, UI primitives.
- Only the web SPA → `apps/web/src/core/*`. Theme, keyboard registry, canvas plumbing, the rAF loop.

**Do not promote a feature to a package speculatively.** Promote it when a second consumer actually
exists. A package that only one app imports is a folder with extra ceremony.

---

## 2. Dependency Direction

One-way flow: `features → core → shared`, never the reverse.

- `shared` imports nothing but third-party packages and workspace packages. If it needs CS2
  knowledge, it is not shared — move it into `core` or the feature.
- `core → core` and `core → shared` are fine. `core` never imports from `features`.
- `features → features`: only through the other feature's barrel, and only downward:

  ```
  library → review → timeline → filters
  radar, controls, settings are leaves
  ```

  There is no `inspector/` slice. #113 removed the column and the drawer both —
  selecting a player expands that player's row inside its team card — so what the slice held now
  lives on the cards in `review/`, and the event feed §5.4 still owes the screen lands there too.

  Never create a cycle. If two features need the same thing, it belongs in `core` or `demo-core`,
  not in a sideways import. Changing this graph is an explicit decision, not a side effect of a PR.
- **Inside a folder, use relative imports; from outside, import the barrel** (`@/features/timeline`,
  `@/core/playback`, `@disa/demo-core`). Never deep-import another slice's internals — the barrel is
  the contract, everything behind it is free to change.

---

## 3. The Pure Core Is Sacred

Two things are pure by contract, and both are load-bearing.

### `packages/demo-core`

- **No React, no DOM, no browser APIs, no `@/` alias, no I/O** — not even `shared`. Plain
  TypeScript only. It has to run unchanged in a Worker, in Node, and in a Tauri process.
- **Read-only logic lives in `helpers/selectors.ts` (queries) and `helpers/predicates.ts`
  (filter and can-do checks). The UI shares these — never re-derive a rule in a component.**
  If a component computes "was this a trade kill", the rule now exists in two places and they will
  drift.
- **No user-visible text.** Ever. Errors are codes (§6). Anything the analytics layer generates for
  display — highlight descriptions, filter summaries, round narratives — is emitted as
  `{ key, params }` i18n data and rendered by the client:

  ```ts
  // demo-core produces this
  { key: 'highlight.wallbangThroughSmoke', params: { attacker: 3, victim: 7, weapon: 'AK-47' } }
  ```

  This is the single easiest rule to break and the most expensive to fix later, because the
  descriptions are the most valuable content in the product.

### `crates/demo-parser`

- **No `wasm-bindgen`, no `js-sys`, no `web-sys`.** Compiles and tests on the host with
  `cargo test`, with no WASM toolchain involved. Platform wrappers are thin and hold every binding.
- **Parsing is deterministic.** The same demo bytes plus the same `SCHEMA_VERSION` produce
  byte-identical output. This is not a stylistic preference: the OPFS cache is keyed on
  `${fileHash}:${SCHEMA_VERSION}`, and golden snapshots are the parser's test suite. A
  non-deterministic parser makes both of them lie. No iteration over hash maps without an explicit
  sort, no timestamps, no thread-order-dependent output.
- `#![forbid(unsafe_code)]` unless a documented, benchmarked reason exists.
- Errors are typed with `thiserror` and map onto the same `ErrorCode` union the TS side consumes.
- `clippy -D warnings` in CI, `rustfmt` on commit.

---

## 4. Comments

**Aim for zero.** Name things so the code reads without them. If code needs a comment to be
readable, fix the code — extract a named function, rename a variable, split the branch.

A comment is justified only for a constraint the code cannot express:

| Justified | Why |
|---|---|
| Engine-derived constants | The number is unverifiable from the code alone |
| Binary layouts and bit-packing | The bit meaning exists nowhere else |
| The WASM boundary | Non-obvious ownership and transfer semantics |
| Workarounds for platform bugs | The next reader will otherwise "simplify" it back |
| Deliberate non-obvious performance choices | Otherwise it reads as a mistake and gets refactored |

```ts
// Source engine free-field footstep audibility. Verified against soundscape data — docs/AUDIO.md.
const FOOTSTEP_RADIUS_UNITS = 1100;

// flags bitfield — must stay in sync with crates/demo-parser/src/export.rs::write_flags
const FLAG_ALIVE = 1 << 0;
const FLAG_DUCKING = 1 << 1;
```

Never write comments that restate the code, narrate history ("used to use X"), or reference tickets,
phases, or PRs.

**No `TODO` comments.** Every piece of work has an issue (`CONTRIBUTING.md` §1), which makes a TODO
a second, worse tracker that no one reads. Open the issue and link it from the PR instead.

**JSDoc** only on exported members of `packages/*` and public functions of the core crate, and only
where the signature is not self-explanatory. Not on internals. Not on React components.

---

## 5. Naming

- `camelCase` values and functions, `PascalCase` types and components,
  `SCREAMING_SNAKE_CASE` module constants. Rust follows `rustfmt` defaults.
- **Units live in the name.** `radiusUnits`, `durationMs`, `speedUnitsPerSec`, `sizeBytes`.
- **`tick` and `frame` are different things and never interchangeable.** `tick` is a demo tick at
  the demo's tick rate; `frame` is a sample index in `TickTrack` at `sampleHz`. A variable named
  `tick` holding a frame index is a bug even when the code works.
- Booleans read as assertions: `isAlive`, `hasDetonated`, `canHearEnemy`. Never `flag`, `status`.
- No abbreviations except established domain ones (`hp`, `nade`, `ct`, `t`, `utility`).
- Event handlers: `handleX` for the implementation, `onX` for the prop.
- Files: `kebab-case.ts`; React components `PascalCase.tsx`. One primary export per file.

---

## 6. Types and Errors

- `strict: true`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`.
- **No `any`.** `unknown` at boundaries, narrowed explicitly.
- **No non-null assertions (`!`)** in application code. Narrow, or throw with a real message.
- No `as` to silence an error. `as const` and narrowing after a real check are fine.
- Model impossible states out of existence:

```ts
// Rejected: four fields that must be read in the right order and can contradict each other.
interface ParseState { isLoading: boolean; progress?: number; result?: ParsedDemo; error?: Error }

// Correct.
type ParseState =
  | { status: 'idle' }
  | { status: 'parsing'; phase: ParsePhase; percent: number }
  | { status: 'ready'; demo: ParsedDemo }
  | { status: 'failed'; error: ParseError };
```

- Branded types for indices that are easy to swap:
  `type Frame = number & { readonly __brand: 'Frame' }`.
- Types describing parsed demo data live in `packages/demo-core/src/schema.ts`. Nowhere else.

### Errors

Never swallow one. No empty `catch`, no `catch { console.log(e) }` as a resting state.

```ts
// The vocabulary itself is `ERROR_CODES` in `packages/demo-core/src/errors.ts`, one identifier per
// variant of the Rust enum and kept in step with it by `bun run errors:check`. Never restate it.
import type { ErrorCode } from '@disa/demo-core';

class DemoParseError extends Error {
  readonly code: ErrorCode;

  constructor(code: ErrorCode) {
    super(code);
    this.code = code;
  }
}
```

The UI maps `code` to translated copy. It never string-matches on `message` and never shows a raw
exception. Throw for programmer errors; return a result for expected failures. A corrupt demo is
expected; a mis-indexed typed array is not.

`ErrorCode` says what a demo turned out to be, and nothing else belongs in it. **Cancellation is not
a code** — an aborted parse terminates its worker and sends no message at all (`AGENTS.md` §7.3),
and out-of-memory is the tab dying rather than a value anything gets to return. A failure that is
the app's rather than the parser's — a saved demo whose cached file has gone — is a union of the
app's own, and `OpenFailure` in `apps/web/src/core/parsing` is what that looks like.

---

## 7. Functional Style

- Prefer small, pure, named functions over inline logic. When a component grows a nontrivial
  computation, extract it into the slice's `helpers/` and unit test it there.
- **No classes** except `Error` subclasses. No class components, no service classes, no singletons
  dressed as classes.
- **Derive, don't duplicate.** Compute view data from the parsed demo on render instead of mirroring
  it into React state. A mirror is a cache, and a cache needs invalidation you did not write.
- Guard clauses over nested conditionals; maximum nesting depth of 3.
- Maximum ~4 parameters, then a named options object.
- Files stay under ~300 lines. Beyond that there are two concerns hiding in one file.
- Export what is used. No barrel that re-exports everything from everywhere — slice barrels export
  the slice's public surface, deliberately chosen.

---

## 8. React

- Function components only. No `forwardRef` unless a ref is genuinely needed.
- **No `useEffect` for derived state.** Compute during render. Effects synchronise with things
  outside React: the worker, the rAF loop, subscriptions, the DOM.
- Every effect has cleanup where one is possible. Every worker terminated, every listener removed,
  every rAF cancelled.
- No `useMemo` / `useCallback` by reflex. Add them when profiling shows a need, or when the value is
  an effect dependency that must not change.
- Props typed inline or with a local `Props` type. No `React.FC`.
- Component files hold the component. Logic testable without a DOM belongs in `helpers/` or
  `demo-core` and is imported.
- Components live in `packages/ui/src/components` and are imported from `@disa/ui` — never from a
  deep path, `AGENTS.md` §2 rule 13. They started as shadcn copies and are **owned** now, so
  edit them there. **Colour and radius still come from the token layer**, and
  a component that hardcodes either is a defect. Size is the exception the ownership buys: `Button`
  and `Input` carry the two heights `tokens.css` allows and no caller sets a height of its own.

---

## 9. Hot-Path Code

Applies to the rAF loop, the renderer, scrub handlers, and any function touching `TickTrack`. These
rules override normal readability preferences, and this is one of the few places a `// perf:`
comment is required.

- **No allocation inside the loop.** No object or array literals, no closures, no
  `.map`/`.filter`/`.reduce`, no spread. Reuse preallocated scratch buffers.
- No optional chaining or destructuring on hot per-frame data. Index the typed array directly.
- No `async`, no `await`, nothing that can suspend.

```ts
// perf: called 60×/s across 10 slots — no allocation, no closures, no method dispatch.
function readPositions(track: TickTrack, frame: number, out: Float32Array): void {
  const base = frame * track.slotCount;
  for (let slot = 0; slot < track.slotCount; slot++) {
    const i = base + slot;
    const o = slot * 3;
    out[o] = track.posX[i];
    out[o + 1] = track.posY[i];
    out[o + 2] = track.posZ[i];
  }
}
```

Everything outside the hot path is written for clarity first. Do not spread these constraints into
ordinary code — premature micro-optimisation elsewhere is its own defect.

---

## 10. Internationalisation

Policy is in `AGENTS.md` §11. These are the patterns.

### Two APIs, deliberately

`t()` is the primitive; `<Text>` is an ergonomic wrapper over it for JSX. Both are needed — a
component cannot produce an `aria-label`, a `document.title`, or a toast string.

```tsx
<Text path="filters.blindDuration.label" as="label" />
<Text path="library.saved.rounds" values={{ count: 7 }} />

const t = useT();
<button aria-label={t('controls.togglePlayback')} />
toast.error(t(errorKeyFor(error.code)));
```

`<Text>` takes `as` (default `span`), `path`, and optional `values`. Nothing else — it is not a
styling component, or styling starts hiding inside it.

### Keys are typed

`path` is a generated union of valid key paths, not `string`. A typo fails at compile time.
**If you are casting to make a key type-check, the key does not exist — add it to `en` first.**

Keys are semantic paths, never the source text:

```ts
'filters.blindDuration.label'      // correct
'Blind duration'                    // rejected — source text is not an identifier
```

### Sentences are whole

```tsx
// Rejected — grammatically impossible in Russian
<Text path="stats.killedPrefix" /> {count} <Text path="stats.enemiesSuffix" />

// Correct — one key, ICU plural
<Text path="stats.killedEnemies" values={{ count }} />
```

```json
// en
{ "killedEnemies": "Killed {count, plural, one {# enemy} other {# enemies}}" }
// ru — all four forms are required
{ "killedEnemies": "Убито {count, plural, one {# врага} few {# врагов} many {# врагов} other {# врага}}" }
```

A fragment that has to *render* differently — a file path carried as game vocabulary, say — is
still one message. `<Text>` takes a node as a value for that reason:

```tsx
<Text path="library.open.hintFolder" values={{ folder: <code>{folder}</code> }} />
```

`useT` keeps the narrower value type on purpose: an `aria-label`, a `document.title` or a toast
cannot hold a node.

Key parity between `en` and `ru` is enforced by a test, not only by the CI script — so it fails in
the same run as everything else.

### Formatting is not translation

`Intl.NumberFormat` / `Intl.DateTimeFormat` with the active locale. Never hand-format, never put a
formatted number inside a translated string.

### Errors

`ErrorCode` maps to a key in the `errors` namespace through one exhaustive function, so adding a
code without copy is a compile error:

```ts
function errorTitleKey(code: ErrorCode): TranslationKey {
  switch (code) {
    case 'POV_DEMO_UNSUPPORTED': return 'errors.povDemo.title';
    case 'MALFORMED_DEMO':       return 'errors.malformed.title';
    // no default — exhaustiveness is the point
  }
}
```

`apps/web/src/features/library/helpers/error-copy.ts` is the whole of it, and it switches on the
stem rather than on the key so the title and the hint cannot name two different codes.

### Game vocabulary

Weapon names, map names, callouts and domain shorthand are **not** translation keys — they are
canonical constants in `demo-core`. Rendering `AK-47` or `Mirage` through `<Text>` is a mistake, not
thoroughness. See the table in `AGENTS.md` §11.

---

## 11. Async and Workers

- `async`/`await` only. No raw `.then()` chains, no promise constructors except when wrapping a
  genuinely callback-based API.
- Every operation that can hang takes an `AbortSignal`. Parsing is cancellable end to end.
- Worker message types are defined once in `packages/demo-parser` and shared by both sides. No
  inline object literals as messages.
- Transfer `ArrayBuffer`s; never structured-clone large payloads. No `postMessage` in a loop — batch.

---

## 12. Tests

- Vitest, `__tests__/` next to the code, `*.test.ts`, **node environment by default.**
  That means only pure code is testable — **this is the point, not a limitation.** It is the
  forcing function that keeps logic out of components. If you want it tested, extract it.
- `happy-dom` only where a component genuinely needs a DOM, and treat that need as a smell worth a
  second look.
- Prefer many small unit tests of one helper (`predicates.blind.test.ts`,
  `selectors.tradekills.test.ts`) over broad scenario tests. Keep scenario tests for cross-cutting
  behaviour: cache invalidation on `SCHEMA_VERSION` change, cancellation mid-parse, round boundaries.
- **Use the builders in `demo-core/__tests__/helpers.ts`** — `newTrack`, `atFrame`, `withKill`,
  `withGrenade` — instead of hand-rolling states or needing a real demo. Almost every test should
  run against a synthetic `TickTrack` measured in kilobytes.
- Coordinate transforms, tick↔frame conversion and filter predicates get exhaustive edges: frame 0,
  last frame, out of range, empty match, single round, 128-tick demo.
- Every bug fix gets a regression test reproducing the bug first.
- `cargo test -p demo-parser` covers the parser core with no WASM toolchain.
- No snapshot tests of React output. Golden snapshots of *parsed demo data* are the exception and
  are reviewed by hand whenever they change.
- No mocking of things you own. If a mock is needed to test a unit, the seam is in the wrong place.

---

## 13. Workflow

```bash
bun run typecheck   # tsc over app, worker, and node configs
bun run check       # biome — lint + format (check:fix to apply)
bun run i18n:check  # key parity, every key read + regenerate the typed key union
bun run test        # vitest
bun run build       # tsc -b && vite build
cargo test -p demo-parser
```

All of these must pass before a push. **Lefthook** (`lefthook.yml`) enforces part of that locally,
so CI is the second line of defence rather than the first:

- **pre-commit** — Biome over the staged files only, and `bun run typecheck` when the commit touches
  `*.ts`/`*.tsx`. `tsc` is a whole-project check by nature: the staged files decide *whether* it
  runs, never *what* it checks. Skipped during a `rebase`, which only replays commits that were
  already checked; not skipped during a `merge`, where the conflict resolution is new code. A commit
  that stages Rust also runs `cargo fmt --check` and `cargo clippy` — the same two lines `wasm.yml`
  runs, so the workflow stops being the first thing to see an unformatted file.
- **pre-push** — `bun run test`.

Hooks install themselves on `bun install` (`trustedDependencies` in the root `package.json`); there
is no manual setup step. Skipping a hook deliberately is documented in `CONTRIBUTING.md` §5 — say so
in the PR when you do.

CI runs the full set and blocks deploys otherwise.

Commits and PR titles follow `CONTRIBUTING.md` §4.

---

## 14. Review Checklist

- [ ] Would a reviewer need to ask what any line does?
- [ ] Any comment that only restates the code — removed? Any required comment (§4) missing?
- [ ] Any `TODO` left behind instead of an issue opened?
- [ ] Any `any`, `!`, or unexplained `as`?
- [ ] Any impossible state representable by the types?
- [ ] Any rule re-derived in a component instead of imported from `demo-core`?
- [ ] Any deep import past another slice's barrel? Any new sideways or upward dependency?
- [ ] Any user-visible text produced inside `demo-core` instead of `{ key, params }`?
- [ ] Every user-facing string through i18n, with `en` and `ru` both filled in?
- [ ] Russian plurals covering all four ICU forms wherever a count appears?
- [ ] Any allocation added to a hot path (§9)? Any `await` in a scrub or render path?
- [ ] Every effect, worker, listener and rAF cleaned up?
- [ ] Every user-facing error mapped to translated copy, not a raw exception?
- [ ] Parser output still deterministic? Golden snapshot change intentional and reviewed?
- [ ] Budgets in `AGENTS.md` §16 still met?

---

## 15. Rejected Patterns

- `useEffect` that copies props into state
- `data`, `info`, `item`, `obj`, `temp`, `handleClick2` as names
- A `utils.ts` dumping ground — name modules for what they do
- `console.log` in committed code (a deliberate `console.warn` at a real boundary is fine)
- Commented-out code — git remembers it
- A `TODO` comment instead of an issue
- Defensive `if (!x) return` guards for cases the types already exclude
- `try/catch` around a whole function body to make an error go away
- Deep-importing `@/features/timeline/helpers/spine` instead of the barrel
- A feature importing upward, or two features importing each other
- Re-deriving a game rule in a component because importing felt inconvenient
- A hardcoded user-facing string, however small
- Using source text as a translation key; running weapon or map names through `<Text>`
- A key added to `en` without its `ru` counterpart
- `wasm-bindgen` anywhere inside `crates/demo-parser`
- A Rust error crossing into JS as a human-readable string instead of a code
- A package created for code only one app imports
- Adding a dependency for something the platform already does (`DecompressionStream`,
  `structuredClone`, `Intl.NumberFormat`, `AbortController`)
