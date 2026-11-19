# Build plan

Status: draft 0.2, 18 Nov 2026.

## How to use this plan

- Work phase by phase. A phase starts when every task in the previous phase is done or explicitly moved, with the move recorded in progress-tracker.md.
- Task IDs are stable: `P<phase>-<number>`. Never reuse or renumber an ID; add new tasks at the end of a phase.
- A task is done only when its "done when" line is true and its tests pass in CI.
- Each task names the feature (F-number from product-scope.md) it serves.
- Status lives in progress-tracker.md, not here. This file changes only when scope changes.
- AI agents take one task at a time, read its dependencies first, and do not start tasks from a later phase.

## Milestones

| Milestone | Phases | Outcome |
|---|---|---|
| A. Talks | P0, P1 | The app chats with all five providers and keeps sessions |
| B. Edits | P2, P3 | The agent edits a project under permission control with diffs and rewind |
| C. Ships | P4, P5, P6 | Git, GitHub, sandbox, and CI: a phone-only bug fix reaches a merged PR |
| D. Complete | P7, P8, P9, P11 | Linux mode, quality gate, long runs, MCP, skills, memory, and the vault |
| E. Release | P10 | v1.0 on GitHub Releases (`full`) and Google Play (`play`) |

Phase order: P11 runs before P10. Phase IDs are stable and P10 (Release) was written first, so skills and memory took the next free ID and sit before the release in the sequence.

## P0. Foundations

Goal: an empty app that calls into the Rust core, with all repo tooling in place.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P0-01 | Create repo layout from project-structure.md | n/a | Every top-level folder exists; README links docs | n/a |
| P0-02 | Rust workspace with all crates as empty libs | n/a | `cargo build` passes for host, `aarch64-linux-android`, `x86_64-linux-android` | P0-01 |
| P0-03 | `tether-ffi` with UniFFI and one `version()` function | n/a | Kotlin calls `version()` on device and shows it | P0-02 |
| P0-04 | Gradle setup: version catalog, convention plugins, modules | n/a | `./gradlew assembleFullDebug assemblePlayDebug` passes | P0-01 |
| P0-05 | `tether.rust-core` convention plugin: cargo-ndk builds `.so` per ABI and packages it | n/a | Clean build from scratch produces a working APK | P0-03, P0-04 |
| P0-06 | Token pipeline: `tokens.json` to `Tokens.kt` | n/a | Changing a color in JSON changes the app after build | P0-04 |
| P0-07 | `TetherTheme` with light and dark schemes, fonts, type scale | n/a | Theme follows the system setting and the in-app override; checked by hand in both themes. Screenshot tests move to P1-19 | P0-06 |
| P0-08 | Navigation shell with placeholder screens for every route | n/a | All routes reachable; back stack correct | P0-07 |
| P0-09 | CI workflow `ci.yml` with rust, tooling, bindings, and android jobs | n/a | Green on main | P0-05 |
| P0-10 | `tools/slop-lint` first version: word list and patterns over `docs/` and `strings.xml` | F12 | Fails on a fixture with banned words; passes on current docs | P0-02 |
| P0-11 | Smell checks in CI: clippy thresholds, detekt config, file length script | F12 | Thresholds match code-standards.md; the file length script fails on an over-limit file (self test) | P0-09 |
| P0-12 | `CoreClient` event bus skeleton with a test event | n/a | A test event from Rust reaches a Compose screen | P0-03 |
| P0-13 | Fixture proofs that Clippy and detekt fail on an over-limit function | F12 | A fixture crate and a fixture Kotlin file each fail their check in CI | P0-11 |
| P0-14 | Docs site on GitHub Pages, built from `docs/` | n/a | The site deploys on a push to `main`; every link between pages resolves | P0-09 |
| P0-15 | Brand assets: logo, adaptive launcher icon, exported icons | n/a | The launcher shows the logo on a device; PNG exports exist for Play, web, and social cards | P0-04 |
| P0-16 | Release workflow that builds and signs both flavors | n/a | A release build of both flavors is signed in CI; the job fails with a clear message when signing secrets are missing | P0-09 |
| P0-17 | Commit size check in CI and `just commits` | n/a | A 16 file commit fails the check (self test); pull requests run it | P0-09 |
| P0-18 | Mobile UI prototype in `tools/prototype` | n/a | Every screen in ui-rules.md has a frame; Blair has reviewed it in a browser | P0-07 |
| P0-19 | Publish a signed release automatically when the core version changes | n/a | Merging a version bump to `main` publishes a GitHub Release with the signed `full` APK, the `play` bundle, and `SHA256SUMS`, with no manual tag and no draft | P0-16 |
| P0-20 | App name is "Tether" in both flavors, shipped as release `v0.0.2` | n/a | The installed `full` APK shows "Tether" as its label; `v0.0.2` is the latest release | P0-19 |

Testing approach: build checks on every ABI; one instrumented test that loads the native library; the theme checked by hand in both modes (screenshot tests are P1-19); slop and smell fixtures that must fail.

## P1. Providers and sessions

Goal: chat with every provider, stream text, persist and resume sessions, show cost.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P1-01 | `tether-types` message, part, event, usage, error types | F3 | Types compile and export through UniFFI | P0-12 |
| P1-02 | `tether-store` schema 0001 and migrations runner | F3 | Fresh install and re-open both work; migration test passes | P1-01 |
| P1-03 | `tether-secrets` with Keystore wrap and unwrap callback | F19 | Key stored, app restarted, key read back; file contains no plaintext | P0-12 |
| P1-04 | SSE parser and provider trait | F1 | Parser passes recorded-stream tests including split chunks | P1-01 |
| P1-05 | Anthropic adapter: streaming, tool calls, usage, caching headers | F1, F14 | Fixture tests pass; live smoke test passes nightly | P1-04 |
| P1-06 | OpenAI adapter (Responses API) | F1 | Same as P1-05 | P1-04 |
| P1-07 | Gemini adapter | F1 | Same as P1-05 | P1-04 |
| P1-08 | OpenAI-compatible adapter; DeepSeek and OpenRouter configs | F1 | Same as P1-05 for both providers | P1-04 |
| P1-09 | Model catalog fetch, cache, capability flags, pricing | F1, F17 | Picker data shows context, price, badges for each provider | P1-05..08 |
| P1-10 | Provider settings screen: key field, test connection, status | F1 | Each state (ok, bad key, network, blocked) shown correctly | P1-03, P1-09 |
| P1-11 | Session actor and plain chat turn (no tools) | F3 | Message streams; session survives app kill and reopens | P1-02, P1-05 |
| P1-12 | Transcript UI: user message, agent text, Markdown, code blocks | F3 | 500-message session scrolls with no frames over 16 ms | P1-11 |
| P1-13 | Composer: input, send, stop, model chip, attachments row | F3, F18 | Send and stop work; long text wraps up to 40% of screen | P1-12 |
| P1-14 | Status line: model, mode placeholder, context meter, cost | F3, F17 | Values update after each turn | P1-11 |
| P1-15 | Sessions list, rename, search | F3 | Search finds text in old messages | P1-11 |
| P1-16 | Image attachments for capable models | F18 | Image reaches the model; disabled with reason on others | P1-13 |
| P1-17 | Voice input | F18 | Speech fills the composer; editable before send | P1-13 |
| P1-18 | Log redaction layer and S7 test | F19 | Test with all key formats finds zero leaks in logs and export | P1-03 |
| P1-19 | Screenshot tests for the P0 components in light, dark, and 200% font scale | n/a | Roborazzi goldens for `PrimaryButton`, `TetherTopBar`, and the theme pass in CI | P0-07 |
| P1-20 | In-app updates for the `full` build | F19 | Settings > About > Updates checks the latest GitHub Release and shows its notes. In `full` it downloads the APK, verifies it against `SHA256SUMS`, and installs it over the running app; `play` links to Google Play instead. The app also checks at start (at most every 12 hours) and shows a banner on the home screen when a newer version exists | P1-10 |

Testing approach: recorded provider streams as fixtures; contract tests per adapter for text, tool call, error, rate limit, and malformed chunk; nightly live smoke test per provider with cheap models; Compose UI tests for transcript and composer; macrobenchmark for scroll.

## P2. Workspace, tools, diffs

Goal: the agent reads and edits a project, and the user reviews and rewinds changes.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P2-01 | Projects: create empty, import zip, export zip | F2 | Round trip of a zip keeps every file byte-identical | P1-02 |
| P2-02 | Path guard with canonicalization and symlink checks | F4 | Escape test suite (20 cases) all rejected | P2-01 |
| P2-03 | File index, glob, grep with ignore rules | F2, F4 | Grep over 10,000 files under 500 ms p95 on reference device | P2-02 |
| P2-04 | Read tools: read_file, list_dir, glob, grep | F4 | Tool fixture tests pass | P2-03 |
| P2-05 | Edit tools: write, edit, multi_edit, move, delete | F4 | Exact-match failures return clear errors; no partial writes | P2-02 |
| P2-06 | Tool loop in the turn: schema validation, results, truncation | F4 | Agent completes a 5-step read and edit task on a fixture repo | P2-04, P2-05, P1-11 |
| P2-07 | Edit batches and checkpoints with blob store | F9 | Each batch stores prior content; blobs deduplicate | P2-06 |
| P2-08 | Rewind: files, conversation, or both | F9 | All three options restore the expected state in tests | P2-07 |
| P2-09 | Tool call cards in the transcript | F3, F4 | Each tool state renders; output collapses past 24 lines | P2-06 |
| P2-10 | Diff card and diff view with per-hunk Keep and Undo | F9 | Undoing one hunk leaves others intact | P2-07 |
| P2-11 | File tree and file viewer with syntax highlighting from core | F2 | 5,000-line file opens under 300 ms | P2-03 |
| P2-12 | Instructions loading: TETHER.md, AGENTS.md, CLAUDE.md, global | F8 | Order and fallbacks match product-scope.md | P2-06 |
| P2-13 | Project home screen | F2 | Shows branch, changes, last session, executor | P2-01 |

Testing approach: path guard property tests; edit tools against fixture files with edge cases (CRLF, BOM, no trailing newline, binary files); engine integration tests driving a scripted fake provider; UI tests for diff hunk actions.

## P3. Control

Goal: permission modes, approvals, plan mode, commands.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P3-01 | Permission table and rule matching in the engine | F5 | Every cell of the mode table in product-scope.md has a test | P2-06 |
| P3-02 | Approval bar UI with Allow, Deny, Always allow, deny reason | F5 | Rules created by Always allow apply in later turns | P3-01 |
| P3-03 | Mode chip and picker; Bypass with biometric and red status line | F5 | Bypass turns off at session end | P3-01 |
| P3-04 | Plan mode and plan card with Run, Edit, Keep planning | F6 | Plan mode blocks edits in tests; Run switches mode | P3-01 |
| P3-05 | `todo_write` and pinned checklist | F6 | Checklist updates live and collapses to the current step | P2-06 |
| P3-06 | `ask_user` tool with tappable options | F4 | Answer returns to the model as the tool result | P2-06 |
| P3-07 | Slash command sheet and built-in commands | F7 | Every command in product-scope.md works | P3-01 |
| P3-08 | `@` file mention picker | F7 | Mentioned file content reaches the model | P2-03 |
| P3-09 | Custom commands from `.tether/commands/` | F7 | `$ARGUMENTS` substitution works | P3-07 |
| P3-10 | `.tether/config.toml` loading and merge order | F8 | Global, project, session precedence tested | P2-12 |
| P3-11 | `/init` generates TETHER.md as a reviewable diff | F8 | Output passes slop rules for writing | P3-07 |

Testing approach: table-driven permission tests; UI tests for approval flows; scripted fake provider for plan mode and ask_user.

## P4. Git and GitHub

Goal: clone, branch, commit, push, PR, with guards.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P4-01 | GitHub OAuth device flow and token storage | F11 | Sign in on a real account; token survives restart | P1-03 |
| P4-02 | Personal access token alternative | F11 | Token validated and scopes listed | P4-01 |
| P4-03 | Clone public and private repos; shallow option | F2, F11 | 200 MB repo clones shallow on mobile data without crash | P4-01, P2-01 |
| P4-04 | Status, diff, log tools | F11 | Match git CLI output on fixtures | P4-03 |
| P4-05 | Branch and commit tools; `tether/<slug>` default branch | F11 | Commits authored with the user's GitHub name and email | P4-04 |
| P4-06 | Push with guards (default branch, force, remote delete) | F11 | Guards ask in every mode as specified | P4-05, P3-01 |
| P4-07 | PR create and PR card with checks and comments | F11 | PR opens with a slop-checked description | P4-06 |
| P4-08 | Commit message drafting with slop check and edit step | F11, F12 | Banned words are flagged before commit | P4-05, P0-10 |

Testing approach: gitoxide operations against local fixture repos; GitHub API calls against recorded responses; one nightly live test on a dedicated test repo.

## P5. Sandbox

Goal: run Python, JavaScript, and SQLite on the phone inside WASM.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P5-01 | Wasmtime engine, config, module cache | F10 | Engine starts once per process; cached module loads under 20 ms | P0-02 |
| P5-02 | Runtime pack builds for Python, QuickJS, SQLite | F10 | Packs build in CI and produce the signed manifest | P5-01 |
| P5-03 | Pack download, signature and hash checks, precompile on device | F10 | Tampered pack is rejected in test | P5-02 |
| P5-04 | WASI context: `/workspace` mount, env without secrets, stdio streaming | F10 | Script writes a file visible in the workspace; env has no key | P5-01 |
| P5-05 | Limits: memory, epoch time limit, output cap | F10 | Infinite loop stops at the limit; app stays responsive | P5-04 |
| P5-06 | `run` tool and router with sandbox only | F10 | `python -m pytest` on a pure-Python fixture passes | P5-05, P2-06 |
| P5-07 | Run card and run console | F10 | Live output streams; exit code and time shown | P5-06 |
| P5-08 | Sandbox benchmarks in `core/benches` | F10 | Cold start p95 under 100 ms on the reference device | P5-06 |

Testing approach: sandbox escape tests (reading outside `/workspace`, reading env, network without permission); limits tests; benchmark gates in nightly.

## P6. CI executor

Goal: run anything through GitHub Actions and feed results back to the agent.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P6-01 | `tether.yml` workflow template and proposal as a diff | F10 | Workflow accepts a command input and runs it | P4-06 |
| P6-02 | Dispatch, run lookup, polling with backoff | F10 | Run status updates in the transcript | P6-01 |
| P6-03 | Log fetch, trimming, failure parsing to file and line | F10 | Python, Jest, Cargo, and Gradle failures parse on fixtures | P6-02 |
| P6-04 | Router fallback to CI with reasons | F10 | Unsupported sandbox command routes to CI with a stated reason | P6-02, P5-06 |
| P6-05 | CI runs list screen and `/ci` | F10 | Lists last 20 runs with status | P6-02 |
| P6-06 | Milestone check: fix a failing test in a real repo from the phone, push, CI green, PR opened | F10, F11 | Done on the reference device with no PC involved | P6-04, P4-07 |

Testing approach: recorded Actions API responses; parser fixtures per test framework; the P6-06 end-to-end check recorded on video for the release notes.

## P7. Linux mode

Goal: an Alpine userland on the phone for package managers and compilers (`full` flavor).

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P7-01 | Build PRoot and talloc for arm64 and x86_64 as `lib*.so` | F10 | Binaries run from `nativeLibraryDir` on Android 10 through the latest release | P0-05 |
| P7-02 | Root filesystem manifest, download, verify, extract | F10 | Setup completes on Wi-Fi in under 2 minutes on the reference device | P7-01 |
| P7-03 | Base package install and repair flow | F10 | Repair fixes a deleted binary | P7-02 |
| P7-04 | Command runner: bind `/workspace`, env scrub, streaming output, kill | F10 | `npm install && npm test` runs on a fixture | P7-03 |
| P7-05 | Per-project network toggle and first-use prompt | F10 | Network off blocks `apk add` and `npm install` with a clear error | P7-04 |
| P7-06 | Router includes Linux mode | F10 | Package manager commands route to Linux mode when installed | P7-04, P6-04 |
| P7-07 | Linux mode setup screen and `/linux` shell | F10 | Shell accepts input with a terminal keyboard row | P7-04 |
| P7-08 | MCP stdio servers through Linux mode | F16 | A sample stdio server's tools appear and work | P7-04, P9-06 |
| P7-09 | `play` flavor stub and tests | F10 | `play` build has no PRoot binaries and shows "not available in this version" | P7-01 |

Testing approach: device matrix test on Android 10, 12, 14, and the latest release; escape tests (no access to other projects or `files/secrets/`); kill and cleanup tests.

## P8. Quality gate

Goal: code smell and AI slop checks on every edit batch, plus second-model review.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P8-01 | Tree-sitter grammar loading for the bundled language list | F12 | Each grammar parses its fixture file | P2-07 |
| P8-02 | Rule trait, registry, changed-range filtering | F12 | Rules only report findings in changed regions | P8-01 |
| P8-03 | Smell rules with thresholds from code-standards.md | F12 | Each rule has a passing and a failing fixture | P8-02 |
| P8-04 | Slop rules for code | F12 | Fixtures for each pattern in product-scope.md F12 | P8-02 |
| P8-05 | Slop rules for tests | F12 | Deleted-test detection works across an edit batch | P8-02 |
| P8-06 | Slop rules for prose, shared with `tools/slop-lint` | F12 | One data file drives both | P8-02, P0-10 |
| P8-07 | Report on diff card; block-level findings stop commits in Auto | F12 | Agent receives findings and fixes them in a scripted test | P8-03..05, P4-05 |
| P8-08 | Language linter integration in the sandbox where available | F12 | Python linter findings merge into the report | P8-07, P5-06 |
| P8-09 | `/review` with a second model and review card | F12 | Review findings link to file and line | P8-07 |
| P8-10 | Quality settings and `[quality]` config overrides | F12 | Threshold change takes effect without restart | P8-07 |

Testing approach: fixture-per-rule with expected findings; performance test of a 20-file batch under 300 ms; precision check on 10 real open-source pull requests to keep false positives under 10% for warn and block levels.

## P9. Long runs, context, web, MCP

Goal: long tasks finish reliably and cheaply.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P9-01 | `AgentService` foreground service and run notification | F13 | 30-minute run finishes with the screen off (S3) | P2-06 |
| P9-02 | Approval notifications with actions | F5, F13 | Approve from the lock screen continues the turn | P9-01, P3-02 |
| P9-03 | Process-death recovery and resume | F13 | Killing the process mid-tool resumes correctly | P9-01 |
| P9-04 | Network pause and resume with backoff | F13 | Airplane mode for 2 minutes mid-turn, then resumes | P9-01 |
| P9-05 | Compaction, `/compact`, `/context` breakdown | F14 | Auto compaction at threshold; task continues correctly | P2-06 |
| P9-06 | Remote MCP client over Streamable HTTP | F16 | A public test server's tools work with approvals | P2-06 |
| P9-07 | `web_fetch` and provider-native `web_search` | F15 | Search hidden for providers without it | P2-06 |
| P9-08 | Budgets and pause; `/cost` screen with charts | F17 | Agent pauses at the limit; Continue resumes | P1-14 |
| P9-09 | OEM battery guidance screens | F13 | Shown before the first long run, with correct per-OEM text | P9-01 |
| P9-10 | Share sheet intake | F18 | Shared text starts a session in the chosen project | P1-13 |

Testing approach: device tests for background behavior on at least Samsung, Pixel, and one Xiaomi device; fault injection for network loss and process death; compaction tests with a scripted long session.

## P11. Skills, memory, and vault

Goal: the agent loads skills on demand, keeps small capped memory, and uses an Obsidian-compatible vault with a link map. Design is in architecture.md section 14.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P11-01 | `SKILL.md` parser and validator in `tether-skills` | F21 | Fixtures from the Agent Skills spec parse; names that are too long or do not match the folder are rejected | P0-02 |
| P11-02 | Skill discovery: bundled, user, and project locations, precedence, project trust prompt | F21 | A project skill loads only after trust; the precedence test passes | P11-01, P2-06 |
| P11-03 | Skill index in the prompt with a token budget; `skills_list` and `skill_view`; mid-session announcement | F21 | 100 skills fit the budget; adding a skill mid-session leaves the cached prefix unchanged | P11-02 |
| P11-04 | `skill_manage` with staged approval and size advisories | F21 | An agent-created skill appears as a diff; denying it writes nothing | P11-03, P3-02 |
| P11-05 | Skill install from a GitHub URL or zip through quarantine and a static scan | F21 | A fixture skill that tells the agent to send secrets elsewhere is flagged and blocked; a clean one installs after approval | P11-02 |
| P11-06 | Skills screen, `/skills`, enable and disable | F21 | List, view, disable, and approve flows work | P11-04 |
| P11-07 | Prune-only upkeep job | F21 | A skill unused for 14 days is marked stale and archived at 30 days; nothing is deleted | P11-02 |
| P11-08 | `MEMORY.md` and `USER.md` with caps and the `memory` tool | F22 | An over-cap write errors; a duplicate is skipped; injection and credential fixtures are rejected | P1-02 |
| P11-09 | Frozen memory snapshot in the system prompt | F22 | A mid-session write changes the file but not the prompt; the next session shows it | P11-08, P1-11 |
| P11-10 | FTS5 session search and `session_search` | F22 | A phrase from a 1,000-message session is found in under 100 ms on the reference device | P1-02 |
| P11-11 | Vault note model (frontmatter, wikilinks, headings) and the `vault_notes` and `vault_links` index, rebuilt from files | F23 | Delete the index and rebuild it: same results; links to missing notes are marked unresolved | P1-02 |
| P11-12 | Vault tools: search, read, write, append, links, orphans, map | F23 | Each tool passes contract tests; `vault_map` stays inside its token budget | P11-11 |
| P11-13 | Start-of-session injector, `index.md` and `log.md` upkeep, write checks | F23 | The slice stays under budget; a write without `description` is rejected | P11-12, P11-09 |
| P11-14 | Memory and vault screens: `/memory`, `/vault`, notes list, backlinks, list-style map, `/vault lint` | F22, F23 | A hand edit to a memory file shows in the next session; backlinks display | P11-13 |
| P11-15 | Vault export, import, and shared-folder sync through the platform layer | F23 | A note edited in Obsidian for Android appears in Tether and the reverse; conflicts keep both copies | P11-11 |
| P11-16 | Redaction and injection tests for memory, skills, and the vault (extends S7) | F19, F21, F22, F23 | A test with planted keys and planted instructions finds zero leaks and zero executed instructions | P11-04, P11-08, P11-12, P1-18 |

Testing approach: parser fixtures from the Agent Skills spec; property tests for the wikilink parser; a rebuild test for the index; prompt-cache tests that compare the prefix before and after a write; planted-secret and planted-instruction fixtures; sync tests with a fake shared folder.

## P10. Release

Goal: v1.0 meets every success criterion and ships.

| ID | Task | Feature | Done when | Depends on |
|---|---|---|---|---|
| P10-01 | Onboarding flow | F20 | First PR in under 15 minutes in the usability test (S1) | P6-06 |
| P10-02 | Accessibility pass: TalkBack, font scale 200%, contrast, targets | n/a | Checklist in ui-rules.md fully passes | all UI tasks |
| P10-03 | Performance pass against every budget in architecture.md | n/a | All budgets met on the reference device | all |
| P10-04 | Benchmark suite run with two models | n/a | S2 met; results committed | P8-07 |
| P10-05 | Security review against architecture.md section 10 | n/a | Every control has a test or a documented manual check | all |
| P10-06 | Slop and smell sweep of the codebase and all strings | F12 | `slop-lint` and smell jobs pass with no suppressions added in P10 | all |
| P10-07 | Release pipeline: signing, both flavors, GitHub Release, Play internal track | n/a | Tagged build publishes both artifacts | P0-09 |
| P10-08 | Play policy decision for Linux mode (Q1) | F10 | Decision recorded; `play` build matches it | P7-09 |
| P10-09 | Store listing, privacy policy, data safety form | n/a | Approved by Play review | P10-07 |
| P10-10 | Name and trademark check (Q6) | n/a | Decision recorded | n/a |

## After v1

| ID | Task | Release |
|---|---|---|
| N1-01 | Telegram connector on the `Connector` trait | v1.1 |
| N1-02 | Sub-agents for read-only exploration | v1.1 |
| N2-01 | Discord connector | v1.2 |
| N2-02 | Scheduled tasks while charging | v1.2 |
| N3-01 | Google Workspace connector: Drive, Sheets, Docs | v1.3 |
| N4-01 | GitLab | v1.4 |
| N5-01 | iOS app (see mobile.md) | v2.0 |
