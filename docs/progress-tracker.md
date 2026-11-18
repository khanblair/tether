# Progress tracker

Last updated: 18 Nov 2026.

## How to update this file

- Update at the end of every work session, by people and AI agents alike.
- Change task status in the tasks table; never delete rows.
- Add new entries at the top of each log, newest first.
- Dates in `YYYY-MM-DD`. Keep entries to one or two lines; link PRs instead of explaining them.
- A decision that changes a doc gets a decisions log entry and a doc changes log entry.
- If this file grows past 600 lines, move log entries older than 90 days to a dated section at the bottom called "Archive" in a single summary line per month.

## Status values

| Value | Meaning |
|---|---|
| Not started | No work yet |
| In progress | Someone is working on it now; owner set |
| Blocked | Cannot continue; the Notes column says why and on what |
| In review | PR open |
| Done | Merged, "done when" met, tests passing |
| Moved | Moved to a later phase or release; Notes says where and why |
| Dropped | Removed from scope; decisions log says why |

## Current phase

**P0. Foundations: In progress.** The repo is public at github.com/khanblair/tether. CI is green on `main`, the docs site is live, and the app runs on an arm64 emulator. The first release, `v0.0.1`, is published and downloadable from GitHub Releases, signed with the pinned key. Open: the Android smoke test of that APK on a device. Next phase: P1, providers and sessions.

## Phase status

Phases run in numeric order except P11, which runs before P10 (see build-plan.md).

| Phase | Name | Status | Started | Finished |
|---|---|---|---|---|
| P0 | Foundations | In progress | 2026-11-18 | |
| P1 | Providers and sessions | Not started | | |
| P2 | Workspace, tools, diffs | Not started | | |
| P3 | Control | Not started | | |
| P4 | Git and GitHub | Not started | | |
| P5 | Sandbox | Not started | | |
| P6 | CI executor | Not started | | |
| P7 | Linux mode | Not started | | |
| P8 | Quality gate | Not started | | |
| P9 | Long runs, context, web, MCP | Not started | | |
| P11 | Skills, memory, and vault | Not started | | |
| P10 | Release | Not started | | |

## Tasks

Current phase in full; later phases are tracked here once their phase starts. Task definitions are in build-plan.md.

| ID | Task | Owner | Status | PR | Notes |
|---|---|---|---|---|---|
| P0-01 | Repo layout | Blair with Claude | Done | n/a | Every top-level folder exists; root README links the docs |
| P0-02 | Rust workspace with all crates | Blair with Claude | Done | n/a | 17 crates: `tether-types`, `tether-engine`, and `tether-ffi` have code; the other 14 are empty libs. Host build, `cargo ndk` for both ABIs, clippy, and tests pass locally |
| P0-03 | UniFFI with `version()` | Blair with Claude | Done | n/a | Pixel 9 arm64 emulator shows "Version 0.0.1" from the Rust core |
| P0-04 | Gradle setup | Blair with Claude | Done | n/a | `assembleFullDebug` and `assemblePlayDebug` pass. Three modules: `app`, `core-bridge`, `designsystem` |
| P0-05 | cargo-ndk into jniLibs | Blair with Claude | Done | n/a | `CargoNdkTask` builds both ABIs into `app/build/rustJniLibs`. Verified from a first build that had no native libs |
| P0-06 | Token pipeline | Blair with Claude | Done | n/a | `tokens/generate.ts` writes `Tokens.kt` and color XML, checks about 50 contrast pairs in both themes |
| P0-07 | TetherTheme | Blair with Claude | Done | n/a | Light and dark follow the system; in-app override works. Screenshot tests moved to P1-19 |
| P0-08 | Navigation shell | Blair with Claude | Done | n/a | All 10 routes reachable on the emulator; back returns to Projects |
| P0-09 | CI workflow | Blair with Claude | Done | n/a | `ci.yml` has four jobs (rust, tooling, bindings, android). First run on `main` was green. Branch protection requires all four |
| P0-10 | `slop-lint` first version | Blair with Claude | Done | n/a | Fails on the bad fixture, passes on current docs, README, and `strings.xml` |
| P0-11 | Smell checks in CI | Blair with Claude | Done | n/a | Thresholds match code-standards.md. File length self test proves the failure path |
| P0-12 | `CoreClient` event bus skeleton | Blair with Claude | Done | n/a | "Send test event" on the emulator shows the event from Rust; `NativeLibraryTest` passes on the emulator |
| P0-13 | Smell fixtures for Clippy and detekt | Blair with Claude | Done | #132 | Merged; issue #3 closed on its own. Fixture crate and module fail their gates; CI runs both through `tools/check-smell-fixtures.sh` |
| P0-14 | Docs site on GitHub Pages | Blair with Claude | Done | n/a | Live at khanblair.github.io/tether; `pages.yml` deploys on docs, images, tokens, or site tool changes |
| P0-15 | Brand assets and launcher icon | Blair with Claude | Done | n/a | Logo traced to `images/logo.svg`; adaptive and monochrome icons; PNG exports. Blair to check the launcher icon by eye |
| P0-18 | Mobile UI prototype | Blair with Claude | Done | n/a | `tools/prototype`, run with `just prototype`. 102 screens in 12 areas; checked by a Node render test, not yet in a browser by Claude |
| P0-17 | Commit size check | Blair with Claude | Done | n/a | `tools/check-commit-size.sh`, `just commits`, and a `tooling` CI step on pull requests |
| P0-19 | Publish a signed release automatically | Blair with Claude | Done | #134, #136 | Merged; `v0.0.1` published as the latest release, signed with the pinned key. Releases are never pre-releases (#136) |
| P0-16 | Release workflow | Blair with Claude | Done | #134 | Ran on the merge of #134 and published `v0.0.1`; issue #4 closed |

## Decisions log

| Date | ID | Decision | Why | Docs affected |
|---|---|---|---|---|
| 2026-11-18 | D22 | The `full` build updates itself: Settings > About > Updates checks the latest GitHub Release, downloads the APK, verifies it against `SHA256SUMS`, and installs it over the running app. The `play` build links to Google Play. Releases are never marked as pre-releases, because the latest release lookup skips them. Risk: a release is offered to every `full` user as soon as it is published, with no staged rollout; a bad release is fixed by publishing a higher version | Owner's request. Replaces the default of notify only in M-Q2. Task P1-20, issue #135 | mobile, build-plan |
| 2026-11-18 | D21 | Releases publish automatically when the version in `core/Cargo.toml` has no tag, from a merged pull request, with no manual tag and no draft; the release fails unless the APK is signed with the key whose fingerprint is pinned in `TETHER_CERT_SHA256` | Owner's decision: merge, then download and test. A failed release leaves no tag, so the next push retries | development, architecture, project-structure, build-plan |
| 2026-11-18 | D20 | Build workflow: Codex writes core Rust tasks and Haiku subagents translate prototype screens to Compose, each task on its own issue and pull request; a fresh-context Sonnet subagent reviews every pull request before merge, with `@codex review` for security-relevant code; subagents never commit, run only the checks for their own files, and run at most 4 at a time; at most 15 files per commit; pull requests are rebase-merged, never squashed; `main` requires a pull request | Owner's decision. Small reviewed PRs catch bugs that a phase-sized diff hides, and a different model reviews than wrote | code-standards, architecture, development, project-structure, build-plan |
| 2026-11-18 | D19 | The docs site is built from `docs/` by `tools/site/build.ts` and deployed with GitHub Pages | One source of truth; the build fails on a broken link. No new top-level folder | README, project-structure, development, architecture |
| 2026-11-18 | D18 | Toolchain: JDK 21 runs Gradle (code targets Java 17), `compileSdk` 37, AGP 9.5.1, Kotlin 2.4.21, Gradle 9.6.1, NDK 27.1.12297006, Rust 1.99.1 | AndroidX 1.19 needs `compileSdk` 37; these are the versions installed and tested. Replaces "JDK 17" in development.md | development, library-docs |
| 2026-11-18 | D17 | Repo is public at `khanblair/tether` with no license yet; `main` gets branch protection after CI is green | Owner's decision. No license means all rights reserved, so the license is still open (Q5) | README, product-scope |
| 2026-11-18 | D16 | Skills, memory, and the Obsidian-compatible vault ship in v1 (phase P11, before release). Skills use the Agent Skills standard with staged approval for agent writes; memory uses capped files with a frozen snapshot; the vault lives in app storage with zip and folder sync | Owner's selection. Designs adapted from Hermes Agent (MIT) and Obsidian's vault format. The Rust core cannot open shared-storage URIs, hence app storage plus sync | product-scope, architecture, build-plan, project-overview, mobile, project-structure |
| 2026-11-18 | D15 | Palette is Velvet Plum (`#C6F0E4`, `#A7E8D7`, `#64165D`, `#8A176E`) with light and dark themes; the accent is plum in light and mint in dark; dynamic color is off. Logo supplied by the owner, traced to SVG. New top-level folder `images/` | Owner's selection. Replaces the amber direction in ui-tokens.md | ui-tokens, ui-rules, ui-registry, project-structure, mobile |
| 2026-11-18 | D14 | The app version has one source, `version` in `core/Cargo.toml`; Gradle reads it | Removes the chance of the app and core versions differing | development |
| 2026-11-18 | D13 | Application ID `io.github.khanblair.tether` (play) and `.full` suffix (full); Kotlin packages stay `app.tether.*` | Owner's selection. Needs no domain ownership. Replaces `app.tether` | project-overview, development, project-structure |
| 2026-11-18 | D11 | Working name "Tether" | Chosen from shortlist; trademark check pending (Q6) | all |
| 2026-11-18 | D5b | Linux mode ships in v1 (`full` flavor), not after | Owner wants full tooling on day one; Play risk handled by flavors | product-scope, architecture, build-plan, mobile |
| 2026-11-18 | D5 | Tiered execution: WASM sandbox default, Linux mode, GitHub Actions | Fast, isolated default that ports to iOS; Linux for real tooling; CI for anything else | product-scope, architecture |
| 2026-11-18 | D4 | v1 providers: Anthropic, OpenAI, Gemini, DeepSeek, OpenRouter | Owner's selection; two API shapes cover all five | product-scope, architecture, library-docs |
| 2026-11-18 | D9 | Android first (API 29+); iOS in v2 | Owner's selection; API 29 is where app exec rules settle | project-overview, mobile |
| 2026-11-18 | D1, D2, D3 | Rust core, Kotlin and Compose UI, UniFFI bindings; Swift later | Owner's selection; one core for both platforms | architecture, library-docs |
| 2026-11-18 | D0 | Tether is a new project, separate from earlier experiments | Owner's decision; no earlier decisions carried over unless restated here | all |
| 2026-11-18 | D6 | No Tether server; bring your own keys | Phone-only principle; no hosting cost or data custody | architecture, product-scope |
| 2026-11-18 | D7 | Git through gitoxide, no git binary | Pure Rust, no exec needed | architecture, library-docs |
| 2026-11-18 | D10 | `TETHER.md`, with `AGENTS.md` and `CLAUDE.md` fallbacks | Existing repos work on day one | product-scope |
| 2026-11-18 | D12 | Telegram (v1.1), Discord (v1.2), Google Workspace (v1.3) after v1, on a `Connector` trait built in v1 | Owner plans these soon; GitHub proves the interface first | product-scope, architecture, build-plan |

## Open questions

Full list with defaults in product-scope.md section 11 and mobile.md section 15. Summary:

| ID | Question | Owner | Due |
|---|---|---|---|
| Q1 | Linux mode in the `play` flavor? | Blair | P10 |
| Q2 | Agent on the default branch when it is the only branch? | Blair | P4 |
| Q3 | Opt-in crash reports or local export only? | Blair | P10 |
| Q4 | Linux mode base packages | Blair | P7 |
| Q5 | Price model, and the code license (none chosen yet; all rights reserved) | Blair | Before public beta |
| Q6 | Trademark check for "Tether" | Blair | P10 |
| Q7 | Chat connectors: user bots only, or optional relay? | Blair | v1.1 planning |
| Q8 | Google scopes for v1.3 | Blair | v1.3 planning |
| Q9 | Vault in shared storage so Obsidian opens it directly? | Blair | P11 |
| Q10 | Approval defaults for agent-written skills, memory, and vault notes | Blair | P11 |
| M-Q1 | Foreground service type for `play` | Blair | P9 |
| M-Q5 | Vault storage location (same as Q9, platform side) | Blair | P11 |

## Session log

| Date | Who | What happened | Next |
|---|---|---|---|
| 2026-11-18 | Blair with Claude | Opened PR #139 with `AGENTS.md` and `CLAUDE.md`. A Sonnet review found wrong claims about what `just check` and `slop-lint` cover, commands that only run from a subfolder, and missing pointers to code-standards section 13; fixed | Merge #139 after review |
| 2026-11-18 | Blair with Claude | Opened PR #134 for P0-19 (automatic release) after PR #132 merged and issue #3 closed on its own. A Sonnet review found the key could be replaced silently and the version was not validated; both fixed | Merge #134, which releases v0.0.1; download and test the APK; back up the keystore |
| 2026-11-18 | Blair with Claude | Opened PR #132 for P0-13 (smell fixtures), the first task on its own issue and branch, with a Sonnet review before merge | Merge #132 and check that issue #3 closes |
| 2026-11-18 | Blair with Claude | Pushed 18 commits (15 files or fewer each) to `khanblair/tether`; CI green on the first run; Pages deployed; branch protection added with the four CI checks (admins not enforced, no review requirement). Earlier the same day: scaffolded the repo: Rust workspace with UniFFI, Android app with flavors and a Velvet Plum theme, logo and launcher icon, tokens pipeline, slop-lint, CI, release and Pages workflows, docs site. Updated every doc to match, added skills, memory, and the vault (F21 to F23, phase P11). Verified on a Pixel 9 arm64 emulator: core version and event path, navigation, theme switch, instrumented test | P0-13 and P0-16, then P1 |
| 2026-11-18 | Blair with Claude | Defined the idea, made stack, provider, executor, platform, and name decisions; drafted all 14 docs | Review docs; create the repo (P0-01) |

## File changes log

| Date | Files | Change | PR |
|---|---|---|---|
| 2026-11-18 | `AGENTS.md`, `CLAUDE.md`, `docs/README.md`, `docs/project-structure.md`, `docs/progress-tracker.md` | Agent instruction files at the repo root, listed in the structure doc and linked from the docs README | #139 |
| 2026-11-18 | `.github/workflows/release.yml`, `docs/development.md`, `docs/progress-tracker.md` | Every release is published as the latest, never a pre-release; tracker brought up to date | #136 |
| 2026-11-18 | Repo root, `core/`, `android/`, `tokens/`, `images/`, `tools/`, `.github/`, and placeholder READMEs | Created the P0 scaffold. The full list is in project-structure.md under Present now | n/a |
| 2026-11-18 | `.github/workflows/release.yml`, `tools/setup-release-signing.sh`, `justfile` | Automatic release on a version with no tag; keystore and secrets setup script | #134 |
| 2026-11-18 | `docs/*` (14 files) | Created | n/a |

## Doc changes log

| Date | Doc | Change |
|---|---|---|
| 2026-11-18 | README, project-structure | `AGENTS.md` and `CLAUDE.md` added to both trees; the README sends agents to `AGENTS.md` first |
| 2026-11-18 | mobile, build-plan | In-app updates for the full build: P1-20 added, M-Q2 decided (D22) |
| 2026-11-18 | development, progress-tracker | Releases are always the latest, never pre-releases (D21 reworded); P0-16 and P0-19 marked Done after `v0.0.1` published |
| 2026-11-18 | development, architecture, project-structure, build-plan | Release publishes on a version bump; `just release-signing`; P0-19 added, P0-16 reworded |
| 2026-11-18 | architecture, development, project-structure | Smell fixtures for Clippy and detekt (P0-13) |
| 2026-11-18 | code-standards, architecture, project-structure | Pull requests must close an issue (`Closes #N`) or say `No issue`; `pr.yml` checks it |
| 2026-11-18 | project-structure, development, build-plan | Added `tools/prototype` and `just prototype`; P0-18 added |
| 2026-11-18 | code-standards, architecture, development, project-structure, build-plan | Git workflow: rebase merge, 15 file commit limit with its CI check, model review before merge; P0-17 added |
| 2026-11-18 | ui-tokens | Rewritten for Velvet Plum: design direction, every color table, contrast table from the checker, generated outputs, no CSS block copy |
| 2026-11-18 | ui-rules, ui-registry | Amber replaced by the accent; P0 components marked Built; screenshot test rule scoped to P1 on |
| 2026-11-18 | project-structure | Added the Present now tree; target tree updated for build-logic, images, tools, skills and memory crates, workflows |
| 2026-11-18 | architecture | Added the skills and memory crates, data tables, files on disk, CI jobs as built, and section 14 on skills, memory, and the vault |
| 2026-11-18 | product-scope | Added F21 skills, F22 memory, F23 vault, their tools and commands, risks, out of scope items, Q9 and Q10, glossary terms |
| 2026-11-18 | build-plan | Added phase P11 and tasks P0-13 to P0-16 and P1-19; P0 done-when lines match what was built; em dash cells replaced |
| 2026-11-18 | development | JDK 21, tool versions, setup and run flow, commands table with planned markers, application IDs, versioning, release steps, troubleshooting |
| 2026-11-18 | code-standards, library-docs | Hand-wired DI for now, detekt exemptions, libraries and tool versions as built, new open decisions for P11 |
| 2026-11-18 | mobile, project-overview, README | Icon and theme notes, SAF for vault sync, new concepts and decisions D13 and D16, slop list exemptions, site link |
| 2026-11-18 | development, code-standards, architecture, mobile, ui-tokens, ui-registry, build-plan, project-structure | Corrections from an independent review: convention plugin names, Rust lint settings, the P0 main-thread exception, backup exclusions, release assets, token names, tree duplicates |
| 2026-11-18 | All 14 docs | First draft |

## Notes for later

- Consider sub-agents for read-only exploration early in v1.1; the turn engine should not assume one agent per session.
- Scheduled tasks (v1.2) should only run while charging and on Wi-Fi by default.
- Watch for a stable Node-compatible WASI runtime; it would move many JavaScript tasks from Linux mode into the sandbox.
- Revisit local model inference when mid-range phones can run a useful coding model at interactive speed.
- Telegram and Discord connectors only work while the phone runs Tether's service; say so plainly in their setup screens.
- Whether to fork or reuse parts of an open-source harness (Codex CLI, Goose) is not decided. A spike that cross-compiles their core crates for Android would settle it before P2.
- The Android vector logo was traced by hand from a 287 px raster. Replace `images/logo.svg` with the original vector when one exists, then update the two launcher drawables.
