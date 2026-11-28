# Code standards

Status: draft 0.1, 18 Nov 2026. These rules apply to people and AI agents equally.

## 1. Repo layout

Defined in project-structure.md. In short: Rust engine in `core/`, Android app in `android/`, design tokens in `tokens/`, docs in `docs/`. Business logic lives in the Rust core. Kotlin owns UI, platform services, and nothing else.

## 2. General rules

1. **Logic goes in the core.** If iOS will need it, it belongs in Rust. Kotlin may format and arrange data for display; it may not decide permissions, routing, cost, or file changes.
2. **One responsibility per function and file.** If the name needs "and", split it.
3. **No silent failures.** Every error is returned, logged with context, or shown to the user. Never swallowed.
4. **No dead code.** Delete unused code; git has the history.
5. **No secrets in code, tests, fixtures, or logs.** Test keys come from environment variables.
6. **Names say what a thing is or does.** No `data`, `info`, `manager`, `helper`, `utils`, `stuff`, or numbered names (`config2`).
7. **Comments explain why, not what.** If a comment restates the code, delete it.
8. **Small pull requests.** Under 400 changed lines excluding generated files and fixtures, unless the PR description explains why.
9. **Docs change with code.** See docs/README.md.

## 3. Rust rules

- Edition 2024. Stable toolchain from `rust-toolchain.toml`.
- `unsafe_code` is forbidden by a workspace lint in every crate except `tether-ffi` and `tether-linux`, which opt out in their own `[lints]` table. Each `unsafe` block there has a `// SAFETY:` comment explaining the invariant.
- Errors: `thiserror` enums per crate; convert to `CoreError` at the engine boundary. No `anyhow` outside tests, benches, and `tools/`.
- No `unwrap()` or `expect()` outside tests, except `expect` on invariants that cannot fail, with the message stating the invariant.
- No `panic!` in library code. A panic crossing FFI is a bug; `tether-ffi` catches unwinds and converts them to `CoreError::Internal`.
- Async: tokio only. Never block inside async code; file I/O, git, tree-sitter, and Wasmtime compilation go through `spawn_blocking`.
- No global mutable state. Pass dependencies explicitly or through the engine.
- Logging with `tracing`; structured fields, not formatted strings. Never log message bodies, file contents, or keys at info level or above.
- Public items in library crates have doc comments when the name alone does not explain them.
- Clippy: `clippy::all` and `clippy::pedantic` as warnings, with project-level allows listed in `Cargo.toml` with a reason each. CI runs with `-D warnings`.
- Formatting: `rustfmt` defaults plus `rustfmt.toml`.

## 4. Kotlin rules

- Kotlin 2.x with the Compose compiler plugin. JVM target 17.
- Architecture per screen: `Screen` composable, `ViewModel`, immutable `UiState`. Composables receive state and lambdas; they never call the core.
- State: `StateFlow` in ViewModels; `collectAsStateWithLifecycle()` in UI. No `LiveData`.
- The database belongs to the core. Kotlin never opens `tether.db`, and never uses `android.database.sqlite` or `SQLiteOpenHelper`. The core links its own SQLite, and a second SQLite library on the same write-ahead-log database in the same process resets the shared WAL index, which kills the core with SIGBUS. Ask the core through the generated bindings. A unit test (`StatusFromCoreTest`) scans the sources for it.
- Coroutines: structured only; `viewModelScope` or an injected scope. No `GlobalScope`. Core calls run on `Dispatchers.IO` through `CoreClient`.
- Compose: stateless components where possible; `remember` keys explicit; `LazyColumn` items always have stable `key`s; mark UI models `@Immutable` when they are.
- No hardcoded user-facing strings; use `strings.xml`. No hardcoded colors, sizes, or durations; use tokens.
- Dependency injection: a hand-wired `AppModule` while the app is small. Hilt is adopted when the module graph grows past a screenful (decision logged then). Constructor injection only.
- Formatting and lint: ktlint (through the ktlint-gradle plugin, rules in `.editorconfig`) and detekt (`android/config/detekt.yml`). CI fails on any finding. Generated files (`generated/`, `tokens/Tokens.kt`) are excluded.
- Nullable types only where absence is meaningful. No `!!` outside tests.

## 5. Swift rules (v2)

Recorded now so the core API is designed for them: SwiftUI with `@Observable` view models; async/await over the UniFFI bindings; the same ViewModel and UiState split as Kotlin; strings in String Catalogs.

## 6. Styling

- Every color, size, radius, duration, and elevation comes from `designsystem` tokens, generated from `tokens/tokens.json`.
- Components come from `designsystem` or the feature's `components/` folder. A component used by two features moves to `designsystem`.
- Before building a new component, check ui-registry.md. After building one, add or update its entry.
- Spacing uses the 4 dp scale. Values off the scale need a comment explaining why.

## 7. Testing and coverage

| Layer | What to test | Coverage target |
|---|---|---|
| Rust core | Every public function; every state machine transition; every permission table cell | 80% lines per crate; 95% for `tether-engine/permissions.rs`, `tether-workspace/guard.rs`, `tether-secrets` |
| Provider adapters | Contract tests against recorded streams: text, tools, errors, rate limits, malformed data | Every recorded case |
| Kotlin ViewModels | State changes for each user action and event | 70% lines |
| Compose UI | Critical flows: send, approve, deny, diff hunk actions, rewind | Every flow listed in ui-rules.md section 5 |
| Screenshots | Every registry component in light, dark, and 200% font scale | All components with status "Built" |
| Device | Background runs, process death, network loss | Release checklist |

Rules:
- Tests are named for the behavior: `rejects_path_outside_workspace`, not `test_guard_3`.
- One behavior per test. Arrange, act, assert, with blank lines between.
- No sleeping in tests. Use fake clocks and test dispatchers.
- Fixtures are minimal and committed; generated fixtures note how to regenerate them.
- Flaky tests are fixed or deleted within two days, never retried in CI to green.

## 8. Security

- Follow architecture.md section 10 for every change touching keys, files, executors, or network.
- All HTTP through the core's single `reqwest` client with rustls. No other HTTP client.
- Validate every tool argument against its schema before use.
- Any new place that writes logs, exports, or notifications must pass through the redaction layer.
- New downloads of executable content (runtime packs, root filesystems) must be listed in a signed manifest.
- Security-relevant files (see project-structure.md) need a CODEOWNERS review.

## 9. Performance

- Follow the budgets in architecture.md section 11. A PR that touches a budgeted path includes a benchmark or trace result.
- No work on the main thread beyond UI.
- Avoid allocations in hot loops in the core: streaming parsers, search, diffing.
- Never hold whole large files in memory when streaming works.
- Compose: avoid recomposition storms; use `derivedStateOf` for computed values and stable parameters.

## 10. Git workflow

- Trunk-based. `main` is always releasable. Short-lived branches: `feat/<id>-<slug>`, `fix/<id>-<slug>`, `docs/<slug>`, where `<id>` is the build-plan task ID.
- Commit messages: Conventional Commits. `feat(session): add approval bar (P3-02)`. Imperative mood, under 72 characters for the subject. Body explains why.
- Rebase merge, never squash. A squash puts a whole pull request into one commit and breaks the 15 file limit below.
- At most 15 files per commit. `tools/check-commit-size.sh` runs in CI on every pull request and fails when a commit goes over.
- Every PR fills the template: the issue it closes (`Closes #N`, or `No issue: <reason>`), what changed, how it was tested, docs updated, progress tracker updated. The `issue-link` check fails a PR whose description has neither.
- Every pull request is reviewed before merge by a different model than the one that wrote it: Claude Code reviews what Codex wrote, and `@codex review` is requested on every pull request. A fresh-context Sonnet review runs only when the owner asks for one (D28). The owner merges. Security-relevant files need a CODEOWNERS review. GitHub approvals are required once there is a second maintainer.
- Never force push to `main`. Never push release tags by hand: `release.yml` creates `v<major>.<minor>.<patch>` when it publishes a release, and a tag that already exists stops that version from releasing.

## 11. Code smells

### Watch list and limits

| Smell | Limit | Rust check | Kotlin check |
|---|---|---|---|
| Long function | 50 lines (Rust, Kotlin); 80 for composables | Clippy `too_many_lines` | detekt `LongMethod` for 50 lines; `@Composable` functions are exempt from the tool, so the 80-line limit is checked in review |
| Long file | 400 lines excluding tests; 800 for a file that holds only tests (`*_tests.rs`, `*_test.rs`, `*Test.kt`, `*Tests.kt`) | `tools/check-file-length.sh` (skips generated files and test folders; Rust inline tests start at `#[cfg(test)]` and are not counted) | Same script |
| Deep nesting | 3 levels | Clippy `excessive_nesting` | detekt `NestedBlockDepth` |
| Too many parameters | 5 | Clippy `too_many_arguments` | detekt `LongParameterList` (`@Composable` functions and default parameters are exempt) |
| High complexity | Cognitive complexity 15 | Clippy `cognitive_complexity` | detekt `CognitiveComplexMethod` |
| Large struct or class | 12 fields or 20 functions | Review | detekt `TooManyFunctions` for the function count; the field count is checked in review |
| Duplicate code | 10 or more identical lines | `tether-quality` duplicate rule on the repo | Same |
| Magic numbers | Any unexplained literal except 0, 1, -1 | Review | detekt `MagicNumber` |
| Boolean parameter | Any public function with a bool flag | Review | Review |
| Swallowed error | Empty catch or ignored `Result` | Clippy `let_underscore_must_use` | detekt `SwallowedException`, `EmptyCatchBlock` |
| Feature envy | Function mostly using another type's data | Review | Review |
| Primitive obsession | Raw strings for IDs | Newtypes required | Value classes required |

### Review checklist

- [ ] Each new function does one thing and its name says what.
- [ ] No limit above is exceeded, or the PR explains why.
- [ ] Errors are handled where they can be acted on, and nowhere else.
- [ ] No copy-pasted blocks; shared logic extracted once there are three uses.
- [ ] No new `TODO` without a task ID.
- [ ] Tests cover the change and would fail without it.
- [ ] Docs, registry, and progress tracker updated where relevant.

## 12. AI slop

AI slop is output that looks finished but is padded, generic, or hollow. Agents produce it under pressure to look helpful. These rules apply to all code in this repo and are also what the product's quality gate checks in users' code.

### In code

| Slop | What to do instead | Caught by |
|---|---|---|
| Comments that restate the next line (`// increment counter`) | Delete; comment only on why | `tether-quality` restating-comment rule, review |
| Placeholder bodies in non-test code (`todo!()`, `TODO: implement`, `pass`) | Implement it or do not add the function | Slop rule, Clippy `todo` and `unimplemented` |
| Defensive checks for impossible states | Trust types; assert invariants once | Review |
| try/catch or `.ok()` wrapping code that cannot fail, or to hide errors | Handle the specific error or let it propagate | Slop rule, detekt `TooGenericExceptionCaught` |
| Helper functions added but never called | Delete them | Clippy `dead_code`, detekt `UnusedPrivateMember` |
| Imports of packages not in the manifest | Add the dependency through library-docs.md, or remove the import | Slop rule, compiler |
| Generic names (`handleData`, `processStuff`, `utils2`) | Name the actual action and subject | Slop rule, review |
| Over-abstraction: interfaces with one implementation and no test seam need | Concrete type until a second use exists | Review |
| Configuration options nobody asked for | Remove; add when needed | Review |
| Verbose logging of every step | Log decisions and failures | Review |
| Emoji or marketing words in logs, errors, and identifiers | Plain words | Slop rule |
| Large rewrites when a small edit would do | Change only what the task needs | Review, PR size limit |

### In tests

| Slop | What to do instead | Caught by |
|---|---|---|
| Assertions that cannot fail (`assert!(true)`, `assertEquals(x, x)`) | Assert on the behavior under test | Slop test rule |
| Tests with no assertions | Add assertions or delete the test | Slop test rule, detekt `UnitTestWithoutAssertion` equivalent |
| Mocking the unit under test | Mock only its dependencies | Review, slop rule |
| Deleting or skipping tests to make a suite pass | Fix the code or the test, and explain in the PR | Slop rule across the batch, CI test count check |
| Snapshot or golden updates with no explaining code change | Investigate the diff first | Review |
| Tests that duplicate the implementation's logic to compute expected values | Use fixed expected values | Review |

### In writing

Docs, strings, commit messages, and PR text follow the writing rules and slop word list in project-overview.md. Caught by `tools/slop-lint` in CI.

## 13. Rules for AI agents

1. Read the docs in the order given in project-overview.md before changing code.
2. Work on one build-plan task at a time. State the task ID in every commit.
3. Do not start tasks whose dependencies are not done.
4. Do not add a library that is not in library-docs.md. Propose it in the PR description and stop.
5. Do not change files listed as security-relevant without saying so at the top of the PR description.
6. Never delete, skip, or weaken a test to make CI pass. If a test is wrong, explain why in the PR.
7. Never commit generated files by hand-editing them; run the generator.
8. Keep diffs minimal. No drive-by reformatting or renaming outside the task.
9. If a doc and the code disagree, stop and ask, or record the question in progress-tracker.md.
10. Update progress-tracker.md at the end of every session: task status, session log, file changes, doc changes.
11. Do not claim something works without running it. Report test commands and their results.
12. Write commit messages and PR text that pass `slop-lint`.
