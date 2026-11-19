# Project structure

Status: draft 0.2, 18 Nov 2026. Lists every folder and file. Files marked (gen) are generated.

Two views: [Present now](#present-now-p0) shows what exists in the repo today and is kept exact. The sections after it show the target layout. A path in the target layout that is not in Present now is planned for the phase named in build-plan.md.

## Rules for keeping this true

1. Adding, moving, or deleting a file outside a test folder updates this doc in the same pull request.
2. Each entry has a one-line purpose. If you cannot write one, the file probably should not exist.
3. Folders listed here are the only top-level folders. A new top-level folder needs a decisions log entry. (`images/` was added with decision D15.)
4. Generated files are listed so people know not to edit them, and are marked (gen).
5. Repeated patterns (such as one file per provider) are listed in full for v1 and noted for later.

## Present now (P0)

Crates that show only `Cargo.toml` and `src/lib.rs` are empty libraries that hold a doc comment.

```
tether/
├── README.md                  Landing page
├── AGENTS.md                  Rules for coding agents: task flow, commits, done means
├── CLAUDE.md                  Imports AGENTS.md; adds Claude Code workflow rules
├── .editorconfig  .gitattributes  .gitignore  .env.example
├── justfile                   Everyday commands
├── rust-toolchain.toml        Rust 1.99.1 with Android targets
├── package.json  pnpm-lock.yaml   Node tooling: token generator, icon export, docs site
├── docs/                      The 14 project docs
├── images/                    Brand assets
│   ├── logo.svg               Master logo (vector)
│   ├── logo.png               (gen) 1024 px render of logo.svg
│   ├── logo-source.jpg        The original raster logo as supplied
│   └── export/                (gen) Play Store icon, web icons, social preview, Open Graph image
├── tokens/
│   ├── tokens.json            Source of truth for every design value
│   └── generate.ts            Generates Tokens.kt, Android color XML, and the site CSS; checks contrast
├── core/                      Rust workspace
│   ├── Cargo.toml  Cargo.lock  deny.toml  clippy.toml  rustfmt.toml
│   ├── fixtures/smells/       Crate with a function over the line limit; clippy must fail on it
│   └── crates/
│       ├── tether-types/      ids, message, session, text, usage, error, event (CoreEvent); no UniFFI code
│       ├── tether-store/      migrations/0001_init.sql; src/{lib,db,error,migrations,schema_tests}.rs
│       ├── tether-engine/     VERSION, EventSink, Core (emits Ready and Test events)
│       ├── tether-secrets/    SecretStore, SecretRef, Secret, SecretsPlatform trait, errors (src/{lib,error,platform,reference,secret,store,store_tests}.rs)
│       ├── tether-providers/  lib.rs (ChatProvider), request.rs, event.rs, error.rs, sse.rs (SseParser), client.rs (shared HTTP client), anthropic/ (adapter), openai/ (adapter), transport.rs (shared request and stream code)
│       ├── tether-ffi/        UniFFI surface, builds libtether_core.so
│       │   ├── uniffi.toml
│       │   └── src/{lib.rs, callbacks.rs, secrets.rs, views/{mod,event,types,error}.rs, bin/uniffi-bindgen.rs}
│       ├── tether-quality/    data/slop_words.txt and slop_patterns.toml (shared with slop-lint)
│       └── tether-{tools,workspace,vcs,sandbox,linux,ci,connectors,mcp,skills,memory}/
│                              Empty libraries
├── android/
│   ├── settings.gradle.kts  build.gradle.kts  gradle.properties  gradlew  gradlew.bat
│   ├── gradle/{libs.versions.toml, wrapper/}
│   ├── build-logic/           Convention plugins as precompiled script plugins
│   │   └── src/main/kotlin/{tether.android.application, tether.android.library, tether.compose,
│   │                        tether.quality, tether.rust-core}.gradle.kts, AndroidCommon.kt, CargoNdkTask.kt
│   ├── config/{detekt.yml, lint.xml}
│   ├── smell-fixtures/        Module with a function over the line limit; detekt must fail on it (built only with -Ptether.smellFixtures)
│   ├── app/                   build.gradle.kts (flavors play and full; app version read from core/Cargo.toml), proguard-rules.pro
│   │   └── src/{main, full, play, androidTest}   TetherApp, MainActivity, di/AppModule, nav/, shell/ (placeholder screens)
│   ├── core-bridge/           CoreClient, EventBus, KeystoreSecretsPlatform, consumer-rules.pro, (gen) generated/tether_core.kt
│   └── designsystem/          TetherTheme, PrimaryButton, TetherTopBar, FONTS-LICENSE.txt, (gen) tokens/Tokens.kt and token colors, Plex fonts
├── ios/README.md              Plan for v2
├── runtimes/README.md  linux/README.md  bench/README.md   Planned folders, one README each
├── tools/
│   ├── slop-lint/             Rust CLI with its own workspace and tests
│   ├── site/{build.ts, site.css}   Builds the Pages site from docs/
│   ├── prototype/             Clickable mobile UI prototype (HTML, CSS, JavaScript; no build step)
│   └── gen-bindings.sh  gen-icons.ts  check-file-length.sh  check-commit-size.sh  check-smell-fixtures.sh  setup.sh  setup-release-signing.sh
└── .github/
    ├── workflows/{ci.yml, pr.yml, release.yml, pages.yml}
    └── pull_request_template.md  CODEOWNERS  dependabot.yml
```

Not present yet: `LICENSE` (no license chosen, see Q5), `nightly.yml` (nothing to run until P1), `core/benches/`, the rest of `core/fixtures/` (only `smells/` and `provider-streams/` exist), feature modules, `service`, `platform`, the `full` and `play` Linux mode files, and the real code of every Rust crate except `tether-types`, `tether-engine`, `tether-ffi`, `tether-store`, `tether-secrets`, and `tether-providers`.

## Top level

Target layout from here on.

```
tether/
├── README.md                  Repo landing page: what Tether is, how to build, link to docs/
├── AGENTS.md                  Rules for coding agents: task flow, commits, done means
├── CLAUDE.md                  Imports AGENTS.md; adds Claude Code workflow rules
├── LICENSE                    License text (not added yet: no license chosen, see product-scope Q5)
├── .editorconfig              Indentation and line endings for all editors
├── .gitignore                 Ignore build output, local.properties, keystores, .env
├── .gitattributes             Mark generated files and lockfiles for diffs
├── justfile                   Everyday commands (see development.md)
├── rust-toolchain.toml        Pinned Rust version and Android targets
├── .env.example               Template for local test keys; never commit .env
├── package.json               Node tooling manifest (token generator, icon export, docs site)
├── pnpm-lock.yaml             Lockfile for the Node tooling
├── docs/                      The 14 project docs
├── images/                    Brand assets: logo source and exported icons
├── core/                      Rust workspace: the shared engine
├── android/                   Android app (Gradle)
├── ios/                       Placeholder for v2
├── tokens/                    Design token source of truth
├── runtimes/                  Build scripts and manifests for sandbox runtime packs
├── linux/                     PRoot build scripts and root filesystem manifest
├── bench/                     Task suite and benchmark harness
├── tools/                     Repo tooling: slop-lint, smell checks, codegen helpers, docs site
└── .github/                   CI workflows, Pages workflow, templates
```

## docs/

```
docs/
├── README.md                  Entry point and doc index
├── project-overview.md        Short summary, decisions, style rules
├── product-scope.md           Full feature definition
├── architecture.md            System design
├── project-structure.md       This file
├── build-plan.md              Milestones and tasks
├── code-standards.md          How code is written
├── development.md             Setup, run, build, release
├── library-docs.md            Approved libraries
├── ui-tokens.md               Design values
├── ui-rules.md                UI behavior and copy
├── ui-registry.md             Component inventory
├── mobile.md                  Android platform details and iOS plan
└── progress-tracker.md        Live status
```

## core/

```
core/
├── Cargo.toml                 Workspace manifest, shared dependency versions, lint config
├── Cargo.lock                 Locked dependency versions
├── deny.toml                  cargo-deny rules: licenses, banned crates, advisories
├── clippy.toml                Clippy thresholds (function lines, complexity)
├── rustfmt.toml               Formatting rules
├── benches/
│   ├── sandbox.rs             Sandbox cold start and Python hello benchmarks
│   ├── search.rs              Glob and grep over a 10,000-file fixture
│   └── store.rs               Database write and query timings
├── fixtures/
│   ├── smells/                Crate that must fail clippy (see tools/check-smell-fixtures.sh)
│   ├── provider-streams/      SSE streams for the parser and adapter tests, written from the providers' documented formats
│   ├── repos/                 Small sample repos for tests (Python, JS, Rust)
│   └── provider-streams/      Recorded provider SSE streams for adapter tests
└── crates/
    ├── tether-types/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs         Re-exports
    │       ├── ids.rs         ProjectId, SessionId, MessageId, ToolCallId newtypes
    │       ├── message.rs     Role, MessagePart, Message
    │       ├── session.rs     PermissionMode, Executor, Provider, ModelRef, SessionStatus, ToolCallStatus, ConnectionStatus
    │       ├── tool.rs        Tool call and result types (later)
    │       ├── event.rs       CoreEvent enum
    │       ├── permission.rs  ApprovalScope (later)
    │       ├── executor.rs    Run types (later)
    │       ├── text.rs        Text forms (as_str, FromStr) for enums kept as text
    │       ├── usage.rs       UsageSnapshot
    │       └── error.rs       CoreError: a variant per kind, with a retryable flag
    ├── tether-ffi/
    │   ├── Cargo.toml         Builds cdylib libtether_core.so
    │   ├── uniffi.toml        Binding names and package for Kotlin and Swift
    │   ├── build.rs           UniFFI scaffolding
    │   └── src/
    │       ├── lib.rs         Exported TetherCore object
    │       ├── callbacks.rs   EventSink, NotifyPlatform, NetworkPlatform
    │       ├── secrets.rs     SecretsPlatform callback, SecretVault
    │       └── views/         Engine types to FFI view types: event.rs, types.rs, error.rs
    ├── tether-engine/
    │   └── src/
    │       ├── lib.rs         Engine entry point
    │       ├── core.rs        Holds sessions, store, config
    │       ├── session.rs     Session actor and state machine
    │       ├── turn.rs        Turn loop
    │       ├── prompt.rs      System prompt assembly and ordering for caching
    │       ├── permissions.rs Mode table and rule matching
    │       ├── router.rs      Executor routing
    │       ├── checkpoints.rs Edit batches, snapshots, rewind
    │       ├── compaction.rs  Summaries and context accounting
    │       ├── budgets.rs     Cost limits and pause
    │       ├── config.rs      .tether/config.toml loading and merging
    │       ├── instructions.rs TETHER.md, AGENTS.md, CLAUDE.md loading
    │       └── commands.rs    Custom slash command loading
    ├── tether-providers/
    │   └── src/
    │       ├── lib.rs         Provider trait and registry
    │       ├── anthropic.rs   Anthropic Messages API adapter
    │       ├── openai.rs      OpenAI Responses API adapter
    │       ├── gemini.rs      Gemini API adapter
    │       ├── openai_compat.rs Chat Completions adapter shared by DeepSeek and OpenRouter
    │       ├── deepseek.rs    DeepSeek settings over openai_compat
    │       ├── openrouter.rs  OpenRouter settings, headers, model list
    │       ├── sse.rs         Server-sent events parser
    │       ├── catalog.rs     Model list fetch and cache
    │       └── pricing.rs     Cost calculation
    ├── tether-tools/
    │   └── src/
    │       ├── lib.rs         Tool registry and schema export
    │       ├── fs.rs          read_file, list_dir, glob, grep
    │       ├── edit.rs        write_file, edit_file, multi_edit, move, delete
    │       ├── run.rs         run tool (calls the router through the engine)
    │       ├── git.rs         git tools
    │       ├── github.rs      pr_create, ci_run, ci_logs
    │       ├── web.rs         web_fetch, web_search glue
    │       ├── todo.rs        todo_write
    │       ├── ask.rs         ask_user
    │       └── format.rs      Result truncation and formatting
    ├── tether-workspace/
    │   └── src/
    │       ├── lib.rs
    │       ├── guard.rs       Path guard
    │       ├── index.rs       File index
    │       ├── search.rs      Glob and grep with ignore rules
    │       └── watch.rs       File watcher
    ├── tether-vcs/
    │   └── src/
    │       ├── lib.rs
    │       ├── clone.rs       Clone and fetch
    │       ├── status.rs      Status and diff
    │       ├── commit.rs      Stage and commit
    │       ├── branch.rs      Branch operations
    │       ├── push.rs        Push with guards
    │       └── auth.rs        Token credentials for HTTPS
    ├── tether-sandbox/
    │   └── src/
    │       ├── lib.rs
    │       ├── engine.rs      Wasmtime engine config and module cache
    │       ├── packs.rs       Runtime pack install, verify, precompile
    │       ├── wasi.rs        WASI context: mounts, env, stdio
    │       ├── limits.rs      Memory, time (epoch), output caps
    │       └── commands.rs    Map program names to pack modules
    ├── tether-linux/
    │   └── src/
    │       ├── lib.rs
    │       ├── rootfs.rs      Download, verify, extract, repair
    │       ├── proot.rs       Command line builder for libproot.so
    │       ├── process.rs     Spawn, stream output, kill
    │       └── packages.rs    apk package installs
    ├── tether-ci/
    │   └── src/
    │       ├── lib.rs
    │       ├── workflow.rs    tether.yml template and checks
    │       ├── dispatch.rs    Workflow dispatch and run lookup
    │       ├── poll.rs        Status polling with backoff
    │       └── logs.rs        Log fetch, trimming, failure parsing
    ├── tether-quality/
    │   ├── data/
    │   │   ├── slop_words.txt     Banned words for writing and identifiers
    │   │   └── slop_patterns.toml Regex and AST patterns for slop
    │   └── src/
    │       ├── lib.rs         Gate entry: run rules on an edit batch
    │       ├── parse.rs       Tree-sitter grammar loading
    │       ├── rules/
    │       │   ├── mod.rs     Rule trait and registry
    │       │   ├── smells.rs  Length, nesting, params, duplicates, dead code
    │       │   ├── slop_code.rs  Restating comments, placeholders, fake imports
    │       │   ├── slop_tests.rs Vacuous asserts, deleted or skipped tests
    │       │   └── slop_text.rs  Banned words and patterns in prose
    │       └── report.rs      Findings, severities, summary
    ├── tether-store/
    │   ├── migrations/
    │   │   └── 0001_init.sql  Initial schema
    │   └── src/
    │       ├── lib.rs
    │       ├── db.rs          Store: open, WAL, foreign keys, run migrations
    │       ├── error.rs       StoreError
    │       ├── migrations.rs  Migration list and runner (PRAGMA user_version)
    │       ├── schema_tests.rs Tests for schema 0001 (test only)
    │       ├── queries.rs     Typed queries per table (later)
    │       └── blobs.rs       Blob store and cleanup (later)
    ├── tether-connectors/
    │   └── src/
    │       ├── lib.rs         Connector trait
    │       └── github/
    │           ├── mod.rs     GitHub connector
    │           ├── device.rs  OAuth device flow
    │           └── api.rs     REST and GraphQL calls
    ├── tether-mcp/
    │   └── src/
    │       ├── lib.rs         MCP client
    │       ├── http.rs        Streamable HTTP transport
    │       └── stdio.rs       stdio transport through Linux mode
    ├── tether-secrets/
    │   └── src/
    │       └── lib.rs         Secret refs, wrap and unwrap through the platform callback
    ├── tether-skills/
    │   └── src/
    │       ├── lib.rs
    │       ├── format.rs      SKILL.md frontmatter parse and validation
    │       ├── discover.rs    Bundled, user, and project skill locations and precedence
    │       ├── disclosure.rs  skills_list and skill_view, prompt index with a size budget
    │       ├── manage.rs      skill_manage: create, patch, rewrite, delete, support files
    │       ├── install.rs     Install from GitHub URL or zip through quarantine and a static scan
    │       └── curator.rs     Prune-only upkeep: stale after 14 days, archived after 30
    └── tether-memory/
        └── src/
            ├── lib.rs
            ├── core_memory.rs MEMORY.md and USER.md with size caps and the memory tool
            ├── snapshot.rs    Frozen snapshot for the system prompt
            ├── search.rs      session_search over the FTS5 index in tether-store
            ├── scan.rs        Prompt-injection and credential scan for memory writes
            └── vault/
                ├── mod.rs     Vault root, notes, append-only entries
                ├── note.rs    Frontmatter, wikilinks, headings
                ├── links.rs   Link graph tables, backlinks, orphans, map queries
                ├── injector.rs Small start-of-session slice from index.md, the project map, and log.md
                ├── validate.rs Checks on each write: description present, links resolve
                └── sync.rs    Reconcile with a shared folder through a platform callback
```

Each crate also has `Cargo.toml` and a `tests/` folder (omitted above after the first entries).

## android/

```
android/
├── settings.gradle.kts        Module list
├── build.gradle.kts           Root build config
├── gradle.properties          JVM args, NDK version pin
├── gradle/
│   ├── libs.versions.toml     Version catalog for all Android libraries
│   └── wrapper/               Gradle wrapper, pinned with a checksum
├── gradlew  gradlew.bat       Wrapper scripts
├── build-logic/               Convention plugins shared by modules, as precompiled script plugins
│   └── src/main/kotlin/
│       ├── AndroidCommon.kt                       Shared SDK levels and Java target
│       ├── CargoNdkTask.kt                        Gradle task that runs cargo-ndk
│       ├── tether.android.application.gradle.kts  Application module setup
│       ├── tether.android.library.gradle.kts      Library module setup
│       ├── tether.compose.gradle.kts              Compose compiler and BOM setup
│       ├── tether.rust-core.gradle.kts            cargo-ndk build into app/build/rustJniLibs
│       └── tether.quality.gradle.kts              detekt and ktlint setup
├── config/
│   ├── detekt.yml             Detekt rules and thresholds
│   └── lint.xml               Android lint config
├── smell-fixtures/            detekt must fail on its long function; included only with -Ptether.smellFixtures
├── app/
│   ├── build.gradle.kts       Flavors: full, play; signing configs
│   ├── proguard-rules.pro     Keep rules for JNA and UniFFI (core-bridge/consumer-rules.pro has the same rules)
│   └── src/
│       ├── main/
│       │   ├── AndroidManifest.xml   Activity, service, receivers, permissions
│       │   ├── kotlin/app/tether/
│       │   │   ├── TetherApp.kt      Application class, core startup
│       │   │   ├── MainActivity.kt   Single activity host
│       │   │   ├── nav/Route.kt          Every route
│       │   │   ├── nav/TetherNavHost.kt  Navigation graph
│       │   │   ├── shell/                Placeholder screens until feature modules exist
│       │   │   └── di/AppModule.kt   Hand-wired dependencies
│       │   └── res/
│       │       ├── values/strings.xml  All user-facing strings
│       │       ├── xml/network_security_config.xml  No cleartext
│       │       ├── xml/backup_rules.xml  Backup exclusions
│       │       ├── xml/data_extraction_rules.xml  Device transfer exclusions
│       │       ├── values/themes.xml  Window background from the bg token
│       │       ├── drawable/ic_launcher_foreground.xml  Logo as a vector, gradients included
│       │       ├── drawable/ic_launcher_monochrome.xml  Single color logo for themed icons
│       │       └── mipmap-anydpi/     Adaptive launcher icon and round icon
│       ├── full/
│       │   ├── AndroidManifest.xml   Linux mode components
│       │   └── jniLibs/              (gen) libproot.so, libproot-loader.so per ABI (planned, P7)
│       └── play/
│           └── kotlin/app/tether/flavor/LinuxModeStub.kt  Linux mode unavailable
├── core-bridge/
│   ├── consumer-rules.pro      Keep rules for JNA and the bindings
│   └── src/main/kotlin/app/tether/core/
│       ├── generated/          (gen) UniFFI Kotlin bindings
│       ├── CoreClient.kt       Callbacks to Flows, lifecycle, dispatcher
│       ├── EventBus.kt         SharedFlow of CoreEvent
│       ├── SecretsPlatformImpl.kt  Keystore wrap and unwrap
│       ├── NotifyPlatformImpl.kt   Approval notifications
│       └── NetworkPlatformImpl.kt  Connectivity callbacks
├── designsystem/
│   ├── FONTS-LICENSE.txt       SIL Open Font License for IBM Plex
│   └── src/main/
│       ├── kotlin/app/tether/design/
│       │   ├── tokens/Tokens.kt    (gen) Token values from tokens/tokens.json
│       │   ├── TetherTheme.kt      Theme, color schemes, typography, ThemeMode
│       │   ├── Icons.kt            Icon set (planned)
│       │   └── components/         Shared components (see ui-registry.md)
│       └── res/
│           ├── font/               IBM Plex Sans 400, 500, 600 and IBM Plex Mono 400, 500
│           └── values/ and values-night/tokens_colors.xml  (gen) Token colors for XML themes
├── feature-onboarding/src/main/kotlin/app/tether/onboarding/  Welcome, key, project screens
├── feature-projects/src/main/kotlin/app/tether/projects/      Project list, create, clone, import, home
├── feature-session/src/main/kotlin/app/tether/session/        Transcript, composer, approvals, pickers
├── feature-diff/src/main/kotlin/app/tether/diff/              Diff view, hunks, checkpoints
├── feature-files/src/main/kotlin/app/tether/files/            File tree and viewer
├── feature-runs/src/main/kotlin/app/tether/runs/              Run console, Linux setup, CI runs
├── feature-settings/src/main/kotlin/app/tether/settings/      All settings screens
├── service/src/main/kotlin/app/tether/service/
│   ├── AgentService.kt         Foreground service during turns
│   ├── Notifications.kt        Channels and builders
│   └── ApprovalReceiver.kt     Notification action handler
└── platform/src/main/kotlin/app/tether/platform/
    ├── Biometric.kt            Biometric prompt wrapper
    ├── Speech.kt               Speech recognizer wrapper
    ├── FilePicker.kt           Storage Access Framework helpers
    └── ShareIntake.kt          Incoming share intents
```

Each feature module follows the same inner layout:

```
feature-x/src/main/kotlin/app/tether/x/
├── XScreen.kt          Top-level composable for the route
├── XViewModel.kt       UI state and actions
├── XUiState.kt         Immutable state classes
└── components/         Composables used only by this feature
```

## ios/

```
ios/
└── README.md           Plan for v2: SwiftUI app over the same core, see mobile.md
```

## tokens/

```
tokens/
├── tokens.json         Source of truth for every design value
└── generate.ts         Generates Kotlin and Android color XML; CSS on request; checks contrast. Swift later
```

## runtimes/

```
runtimes/
├── manifest.template.json   Pack list, versions, hashes; signed at release
├── python/build.sh          Fetch and package the CPython WASI build
├── quickjs/build.sh         Build QuickJS for WASI
├── sqlite/build.sh          Build the sqlite3 shell for WASI
└── sign.sh                  Sign the manifest with the release key
```

## linux/

```
linux/
├── proot/build.sh           Cross-compile PRoot and talloc for arm64 and x86_64
├── rootfs-manifest.json     Alpine version, URLs, SHA-256 per architecture
└── base-packages.txt        Packages installed during setup
```

## bench/

```
bench/
├── tasks/                   30 task folders: repo/, prompt.md, check.sh
├── runner/                  Script that drives the app over adb for each task
└── results/                 (gen) Results per release, committed as CSV
```

## tools/

```
tools/
├── slop-lint/               Rust CLI using tether-quality data files
│   ├── Cargo.toml
│   └── src/main.rs          Checks docs, strings.xml, comments, commit messages
├── check-file-length.sh     Fails on files over the limits in code-standards.md
├── check-commit-size.sh     Fails on a commit that touches more than 15 files
├── check-smell-fixtures.sh  Proves Clippy and detekt fail on a function over the line limit
├── setup-release-signing.sh One-time: create the release keystore and store it as GitHub secrets
├── gen-bindings.sh          Regenerates UniFFI Kotlin bindings
├── gen-icons.ts             Renders PNG brand assets from images/logo.svg
├── setup.sh                 Toolchain check behind `just setup`
├── site/
│   ├── build.ts             Builds the Pages site from docs/, tokens, and images
│   └── site.css             Site styles on top of the generated token CSS
└── prototype/               Clickable mobile UI prototype; see its README
```

## .github/

```
.github/
├── workflows/
│   ├── ci.yml               rust, tooling, bindings, android jobs
│   ├── pr.yml               issue-link check on every pull request and description edit
│   ├── nightly.yml          Benchmarks and live provider smoke tests (planned, P1)
│   ├── pages.yml            Builds and deploys the docs site to GitHub Pages
│   └── release.yml          Build, sign, and publish a GitHub Release for both flavors when the core version has no tag
├── pull_request_template.md Checklist including docs and progress tracker
├── dependabot.yml           Weekly dependency update PRs
└── CODEOWNERS               Review owners per folder
```

## Generated files

| File | Generated by | Committed |
|---|---|---|
| `android/core-bridge/.../generated/*` | `tools/gen-bindings.sh` | Yes; CI checks they match |
| `android/designsystem/.../tokens/Tokens.kt` and `android/designsystem/src/main/res/values*/tokens_colors.xml` | `tokens/generate.ts` | Yes; CI checks they match |
| `android/app/src/full/jniLibs/*` | `linux/proot/build.sh` | No; built in CI and by `just linux-libs` |
| `android/app/build/rustJniLibs/*/*/libtether_core.so` | cargo-ndk through Gradle | No (build output) |
| `images/logo.png`, `images/export/*` | `tools/gen-icons.ts` | Yes |
| `site-dist/` | `tools/site/build.ts` | No; deployed by `pages.yml` |
| `bench/results/*.csv` | Bench runner | Yes |

## Where tests go

| Code | Tests |
|---|---|
| Rust crate | Unit tests in the same file under `#[cfg(test)]`; integration tests in the crate's `tests/` |
| Cross-crate engine flows | `core/crates/tether-engine/tests/` with fixtures from `core/fixtures/` |
| Kotlin ViewModels and logic | `src/test/kotlin/` in the same module |
| Compose UI | `src/androidTest/kotlin/` in the same module |
| Screenshot tests | `src/test/kotlin/.../screenshots/` with Roborazzi |
| End-to-end on device | `android/app/src/androidTest/kotlin/app/tether/e2e/` |

## Naming rules

| Item | Rule | Example |
|---|---|---|
| Rust crates | `tether-<area>`, kebab case | `tether-sandbox` |
| Rust files and modules | snake case | `openai_compat.rs` |
| Kotlin packages | `app.tether.<module>` | `app.tether.session` |
| Android application ID | `io.github.khanblair.tether` (play), `.full` suffix (full) | `io.github.khanblair.tether.full` |
| Kotlin files | PascalCase, named after the main declaration | `ToolCallCard.kt` |
| Composables | PascalCase nouns | `DiffCard` |
| ViewModels | `<Feature>ViewModel` | `SessionViewModel` |
| Gradle modules | kebab case, `feature-` prefix for features | `feature-diff` |
| String resource keys | `<screen>_<element>_<purpose>` | `session_composer_hint` |
| Database tables | plural snake case | `tool_calls` |
| Events | PascalCase, past tense or state | `ToolFinished`, `ApprovalRequired` |
| Work branches made by the agent | `tether/<task-slug>` | `tether/fix-login-timeout` |

## Shared and special files

| File | Why it is special |
|---|---|
| `tokens/tokens.json` | Only place design values are defined |
| `images/logo.svg` | Only master of the logo; the Android vector icons are traced from it by hand |
| `android/app/src/main/res/values/strings.xml` | Only place user-facing English text lives |
| `core/crates/tether-quality/data/*` | Shared by the in-app gate and `tools/slop-lint` |
| `core/crates/tether-types/src/*.rs` and `core/crates/tether-ffi/src/views/` | Changing a type the UI sees changes the FFI; regenerate bindings (`just bindings`) |
| `rust-toolchain.toml` and `gradle.properties` | Toolchain pins; change only with a decisions log entry |
| `runtimes/manifest.template.json` and `linux/rootfs-manifest.json` | Security-relevant; changes need review from CODEOWNERS |
