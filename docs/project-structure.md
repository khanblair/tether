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
│   ├── fixtures/model-lists/  Provider list responses for catalog tests
│   ├── fixtures/provider-streams/  Recorded provider SSE streams for adapter tests
│   └── crates/
│       ├── tether-types/      ids, message, model catalog, session, text, usage, error, event (CoreEvent); no UniFFI code
│       ├── tether-store/      SQLite storage: schema, migrations, sessions, projects, checkpoints, approvals, budgets, MCP servers, skills
│       │   ├── migrations/{0001_init,0002_message_fts,0003_checkpoints,0004_approvals,0005_prompt_snapshot,0006_session_todos,0007_project_trust,0008_project_network,0009_git_identity,0010_budgets,0011_context,0012_mcp_servers,0013_skills_memory,0014_session_reasoning,0015_session_archive}.sql
│       │   └── src/{lib,db,error,migrations,models,projects,sessions,session_rows,session_load,session_search,session_archive,session_fts,session_context,session_plans,checkpoints,approvals,permission_rules,project_trust,project_network,prompt_snapshots,provider_configs,todos,usage,budgets,context,mcp_servers,skill_settings}.rs; test modules (*_tests.rs, tests.rs)
│       ├── tether-engine/     Sessions, turn loop, permissions, executors, tool loop, checkpoints, prompts, config, quality gate, updates
│       │   ├── src/{lib,budget,checkpoint_capture,context,network,notify,provider_connection,rewind,session,session_provider,tool_loop,usage}.rs; folders commands/, config/, executors/, permissions/, prompt/, quality/, session/, updates/; test modules (*_tests.rs, tests.rs)
│       │   └── src/tool_loop/     Folders ci_tools/, git_tools/, knowledge_tools/, mcp_tools/, session_tools/; file_tools.rs, registry.rs, stream.rs
│       ├── tether-secrets/    SecretStore, SecretRef, Secret, SecretsPlatform trait, errors (src/{lib,error,platform,reference,secret,store,store_tests}.rs)
│       ├── tether-providers/  lib.rs (ChatProvider, ModelCatalog), catalog.rs, pricing/ and data/prices/, request.rs, event.rs, error.rs, sse.rs (SseParser), client.rs (shared HTTP client), anthropic/ (adapter), openai/ (adapter), gemini/ (adapter), openai_compat/ (adapter), deepseek.rs, openrouter.rs, transport.rs (shared request and stream code)
│       ├── tether-ffi/        UniFFI surface, builds libtether_core.so
│       │   ├── uniffi.toml
│       │   └── src/{lib,approvals,archive,callbacks,ci,ci_service,ci_types,commands,config,context,executors,executors_download,executors_http,files,git,git_clone,git_support,git_types,github,knowledge,mcp,mcp_support,mcp_types,memory,network,notify,projects,provider_settings,quality,questions,resume,secrets,session_control,sessions,skills,skills_install,updater,usage,vault,vault_archive,vault_manifest,vault_sync}.rs; folders sessions/ (open, prompt, provider, recovery, user_runs, ux), skills_install/ (api, download, github_url, preview, quarantine, scope, tree, upkeep, views), views/ (approval, checkpoint, context, error, event, event_from, plan, project, question, session, todo, types, usage), bin/ (uniffi-bindgen); test modules
│       ├── tether-quality/    Quality gate: smell and slop rules, prose checks, review parsing, highlighting
│       │   ├── data/{generic_names.txt,slop_patterns.toml,slop_words.txt} (shared with slop-lint)
│       │   └── src/{lib,highlight,language,manifest,model,prose,registry,report,review,thresholds}.rs; rules/{mod,slop,smell,smell_text,tests_rules,util}.rs; tests/ (integration tests, Markdown and Rust fixtures)
│       ├── tether-workspace/  Project workspace: path guard, file index, ignore rules, ZIP archives
│       │   └── src/{lib,file_index,ignore_rules,path_guard}.rs; archive/; test modules (*_tests.rs)
│       ├── tether-tools/      Agent tools: guarded read and atomic edit
│       │   └── src/{lib,edit,fs}.rs; test modules in fs/ and edit/
│       ├── tether-skills/     Parse, validate, discover, scan, and prune Agent Skills (SKILL.md)
│       │   └── src/{lib,discovery,index,scan,scan_rules,upkeep}.rs; discovery/file_ops.rs; test modules (*_tests.rs, tests.rs)
│       ├── tether-memory/     Capped notes, frozen snapshot, Markdown vault view, vault tools, sync contract
│       │   └── src/{lib,files,snapshot}.rs; vault/{mod,sync,tools}.rs; test modules in files/, vault/
│       ├── tether-vcs/        Git and GitHub: clone, local repo operations, smart HTTP push, GitHub API, OAuth, PAT check
│       │   └── src/{lib,clone,oauth,pat,repo,test_support}.rs; folders repo/ (checkout, diff, helpers, log, ops, stageable), push/ (mod, http, protocol), github/ (client, helpers, mod); test modules (*_tests.rs, tests.rs)
│       ├── tether-sandbox/    Wasmtime WASI sandbox: module cache, runner, limits, runtime pack install and verify
│       │   └── src/{lib,limits,module_cache,output_pipe,pack,pack_catalog,pack_cleanup,pack_install,pack_io,pack_layout,pack_replace,runner,runner_result,runner_support,runner_validation,test_support,wasi_context}.rs; test modules (pack_tests.rs, runner_tests.rs)
│       ├── tether-linux/      Linux mode: PRoot runs, Alpine root filesystem, apk packages, PTY shell
│       │   └── src/{lib,error,extract,manifest,packages,proot,rootfs,rootfs_io,runner,shell}.rs; tests/ (integration and hardening tests)
│       ├── tether-ci/         GitHub Actions client, workflow file, failure log parser, run poller
│       │   └── src/{lib,client,logs,poll,workflow}.rs; folders client/ and logs/ (support.rs each); tests/ci_tests.rs
│       ├── tether-connectors/ Web fetch, HTML to text, and public IP checks for SSRF protection
│       │   └── src/{lib,backend,content,fetch,html,ip}.rs; hardening_tests.rs (test module)
│       └── tether-mcp/        MCP client for Streamable HTTP servers
│           └── src/{lib,names,protocol,render,session,transport}.rs; test modules tests.rs, hardening_tests.rs
├── android/
│   ├── settings.gradle.kts  build.gradle.kts  gradle.properties  gradlew  gradlew.bat
│   ├── gradle/{libs.versions.toml, wrapper/}
│   ├── build-logic/           Convention plugins as precompiled script plugins
│   │   └── src/main/kotlin/{tether.android.application, tether.android.library, tether.compose,
│   │                        tether.quality, tether.rust-core}.gradle.kts, AndroidCommon.kt, CargoNdkTask.kt
│   ├── config/{detekt.yml, lint.xml}
│   ├── smell-fixtures/        Module with a function over the line limit; detekt must fail on it (built only with -Ptether.smellFixtures)
│   ├── app/                   build.gradle.kts (flavors play and full; app version read from core/Cargo.toml), proguard-rules.pro
│   │   └── src/{main, debug, release, full, play, androidTest}   TetherApp, MainActivity, di/AppModule, nav/, home/ (Projects home, project detail, drawer, Needs attention; sample sources in debug, empty sources in release), share/ (text intake and project picker), shell/ (settings and placeholder screens)
│   │       └── main/kotlin/app/tether/
│   │           ├── session/agent/                  Agent loop state and gateway, agent screen, agent tool cards in the transcript
│   │           ├── session/approval/               Approval bar, always-allow menu, deny reason sheet, approval view model
│   │           ├── session/cards/                  Ask-user card, plan card, checklist pin
│   │           ├── session/tools/                  Tool call and group cards, run card, quality report, system notes, tool titles and icons
│   │           ├── session/notices/                Budget, context, cost, network, and interrupted-turn notices
│   │           ├── session/compose/                Slash commands and their sheet, @mentions and chips, compose assist panel
│   │           ├── session/voice/                  The Listening sheet, the speech recognizer wrapper, and its levels
│   │           ├── git/                            Git screen, changes and branches tabs, diff screen and view, unified diff parser
│   │           ├── projectarchive/                 Project ZIP import and export screens, export sheet, view models
│   │           ├── settings/github/                GitHub account, clone, and Git identity screens, device code panel
│   │           ├── settings/mcp/                   MCP server list, server detail, add sheet, tool rows, MCP gateway
│   │           ├── settings/executors/             Executors screen, Linux setup card, runtime packs, CI runs screen
│   │           ├── settings/budgets/               Budgets and cost screens, budget editor, cost chart, money formatting
│   │           └── workspace/                      ProjectArchive.kt: ZIP project folders
│   ├── core-bridge/           CoreClient, EventBus, KeystoreSecretsPlatform, consumer-rules.pro, (gen) generated/tether_core.kt
│   │   └── src/main/kotlin/app/tether/workspace/ProjectArchive.kt  ZIP project folders
│   └── designsystem/          TetherTheme, TetherIcons, PrimaryButton, TetherTopBar, ListRow, ProviderRow, SecretField, InlineBanner, TestConnectionButton, TetherTextButton, TetherIconButton, NotificationBell, StatusBadge, StatusMark, ProjectRow, SessionRow, SectionTitle, ListGroup, TetherSearchField, TetherSheet, FONTS-LICENSE.txt, (gen) tokens/Tokens.kt and token colors, Plex fonts
├── ios/README.md              Plan for v2
├── runtimes/README.md  linux/README.md  bench/README.md   Planned folders, one README each
├── tools/
│   ├── slop-lint/             Rust CLI with its own workspace and tests
│   ├── site/{build.ts, site.css}   Builds the Pages site from docs/
│   ├── prototype/             Clickable mobile UI prototype (HTML, CSS, JavaScript; no build step)
│   └── gen-bindings.sh  gen-icons.ts  check-file-length.sh  check-commit-size.sh  check-smell-fixtures.sh  bump-version.sh  release-version.sh  setup.sh  setup-release-signing.sh
└── .github/
    ├── workflows/{ci.yml, pr.yml, release.yml, pages.yml}
    └── pull_request_template.md  CODEOWNERS  dependabot.yml
```

Not present yet: `LICENSE` (no license chosen, see Q5), `nightly.yml` (nothing to run until P1), `core/benches/`, `core/fixtures/repos/`, the `runtimes/` build scripts and manifest (only `runtimes/README.md` exists), the `linux/` PRoot build scripts, rootfs manifest, and package list (only `linux/README.md` exists), feature modules (the Gradle build has only `app`, `core-bridge`, and `designsystem`), `service`, and `platform`.

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
│   ├── model-lists/           Provider model list responses for catalog tests
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
    │   └── src/
    │       ├── lib.rs         UniFFI surface of the core: exported TetherCore object
    │       ├── approvals.rs   User-facing approval resolution for session actors
    │       ├── archive.rs     Shared archive limits and FFI error mapping
    │       ├── callbacks.rs   Callback interfaces the platform layer implements
    │       ├── ci.rs          UniFFI runtime for GitHub Actions runs and log views
    │       ├── ci_service.rs  GitHub Actions run and log service behind the CI runtime
    │       ├── ci_types.rs    CI views and the conversion from engine CI failures
    │       ├── commands.rs    UniFFI views for command and mention outcomes
    │       ├── config.rs      UniFFI access to global, project, and trust-aware configuration
    │       ├── context.rs     Context breakdown and compaction controls
    │       ├── executors.rs   UniFFI configuration for local command executors
    │       ├── executors_download.rs  Runtime pack and Linux root filesystem downloads
    │       ├── executors_http.rs      HTTPS fetchers shared by executor installers
    │       ├── files.rs       Read-only project files for the tree, viewer, and search
    │       ├── git.rs         UniFFI runtime for local Git repositories and clone operations
    │       ├── git_clone.rs   Clone runs, cancellation, and the clone factory for the Git runtime
    │       ├── git_support.rs Factories that connect the Git runtime to the engine's Git tools
    │       ├── git_types.rs   Git views, the error enum, and the progress callback
    │       ├── github.rs      GitHub sign-in and identity views
    │       ├── knowledge.rs   FFI settings and staged diff access for knowledge tools
    │       ├── mcp.rs         UniFFI controls for MCP servers and their secret-backed headers
    │       ├── mcp_support.rs MCP store and view adapters
    │       ├── mcp_types.rs   MCP records and errors
    │       ├── memory.rs      Capped memory files and session snapshot
    │       ├── network.rs     Network platform controls
    │       ├── notify.rs      Background notification and foreground-service dispatch
    │       ├── projects.rs    Project creation and listing
    │       ├── provider_settings.rs  Provider key management and connection status
    │       ├── quality.rs     Quality gate settings and built-in rules for the platform
    │       ├── questions.rs   User-facing question resolution for session actors
    │       ├── resume.rs      Resume and discard controls for sessions recovered after process death
    │       ├── secrets.rs     Secret store platform callback and vault calls
    │       ├── session_control.rs    Checklist, plan, and plan execution calls
    │       ├── sessions.rs    Session creation, transcript reads, plain chat turns
    │       ├── sessions/      open.rs, prompt.rs (session prompt sections, global instructions), provider.rs (provider key lookup and factory), recovery.rs (one-time recovery of interrupted sessions), user_runs.rs (user command runs and reruns), ux.rs (saved settings, model catalog, usage, search, archive and delete)
    │       ├── skills.rs      Read-only skill discovery
    │       ├── skills_install.rs  Skill quarantine, installation, and prune-only upkeep
    │       ├── skills_install/    api.rs (install entry points), download.rs (validated codeload ZIP), github_url.rs (URL parsing), preview.rs (quarantine preview), quarantine.rs (quarantine folders), scope.rs (storage roots, name checks), tree.rs (skill tree walk), upkeep.rs (usage scan, archive moves), views.rs (install records)
    │       ├── updater.rs     Check and download full-build updates
    │       ├── usage.rs       Usage and budget runtime
    │       ├── vault.rs       Private Markdown vault operations
    │       ├── vault_archive.rs  ZIP import and export for the vault
    │       ├── vault_manifest.rs Persistent vault sync observations
    │       ├── vault_sync.rs  Platform-folder sync adapter and manifest persistence
    │       ├── views/         Engine types to FFI views: approval, checkpoint, context, error, event, event_from, plan, project, question, session, todo, types, usage
    │       ├── bin/uniffi-bindgen.rs  Binding generator run by tools/gen-bindings.sh
    │       └── *_tests.rs, tests.rs  Test modules (also projects/, updater/, sessions/ test folders)
    ├── tether-engine/
    │   └── src/
    │       ├── lib.rs         Engine entry point: sessions, turn loop, permissions, executor router, checkpoints
    │       ├── session.rs     One actor per session: serializes turns and commits streamed text before emitting it
    │       ├── session/       Actor parts: compact.rs (context compaction), dispatch.rs (command routing), handle.rs (command queue), prompt.rs (session constructors), rewind.rs (checkpoint restore), stream.rs (batched provider events), turn.rs (turn loop)
    │       ├── session_provider.rs  The one provider factory used by a saved session
    │       ├── provider_connection.rs  Low-cost authenticated catalog request that checks each provider key
    │       ├── tool_loop.rs   Bounded provider and tool exchange with saved messages and checkpoints
    │       ├── tool_loop/     file_tools.rs (guarded file handler), registry.rs (prepared tool calls), stream.rs (one provider response)
    │       ├── tool_loop/ci_tools/        GitHub Actions tools for one project
    │       ├── tool_loop/git_tools/       local.rs (status, diff, log, branch), remote.rs (push and pull request), validation.rs (argument checks), mod.rs (Git and GitHub tools for one workspace)
    │       ├── tool_loop/knowledge_tools/ execute.rs (tool execution), support.rs (validation and storage helpers), mod.rs (skill, memory, session search, and vault tools)
    │       ├── tool_loop/mcp_tools/       manager.rs and manager_impl.rs (lazy MCP sessions, tool-list cache), mcp.rs (namespaced MCP tool handler), web.rs (public web fetch tool), mod.rs (MCP and web network tools)
    │       ├── tool_loop/session_tools/   Checklist, question, and plan tools; validation.rs (argument checks)
    │       ├── checkpoint_capture.rs  Guarded snapshots of prior file bytes before an edit batch
    │       ├── rewind.rs      Restore a saved edit checkpoint
    │       ├── permissions/   Permission modes, conservative project rules, pure decisions
    │       ├── executors/     mod.rs (executor entry and built-in router), run.rs (model-facing run tool; run/run_support.rs helpers), router.rs (shell-aware routing; router/tokenizer.rs)
    │       ├── prompt/        Stable system prompt assembly: base.rs, builder.rs, instructions.rs, vault.rs (vault slice)
    │       ├── config/        Project and global TOML config with trust-aware merging: mod.rs, parser.rs, serde_text.rs
    │       ├── commands/      Slash commands: builtins.rs, custom.rs (custom command files), export.rs (session export), mentions.rs (@mentions and size caps)
    │       ├── quality/       Quality gate run for one native file-edit batch
    │       ├── budget.rs      Budget gates that pause a turn before a provider request
    │       ├── context.rs     Context accounting and message shaping for long sessions
    │       ├── network.rs     Network pause state shared by session actors and the platform callback
    │       ├── notify.rs      Short, redacted notification text for tool progress
    │       ├── usage.rs       Provider response pricing and UTC day helpers
    │       ├── updates/       Release check, bounded APK download, SHA256SUMS parsing, version comparison
    │       └── *_tests.rs, tests.rs  Test modules
    ├── tether-providers/
    │   └── src/
    │       ├── lib.rs         Provider trait and registry
    │       ├── anthropic.rs   Anthropic Messages API adapter
    │       ├── openai.rs      OpenAI Responses API adapter
    │       ├── gemini.rs      Gemini API adapter
    │       ├── openai_compat/ Chat Completions adapter shared by DeepSeek and OpenRouter (mod, http, request, messages, stream, wire)
    │       ├── deepseek.rs    DeepSeek names over openai_compat
    │       ├── openrouter.rs  OpenRouter names over openai_compat; its headers live in `OpenAiCompatConfig::openrouter`
    │       ├── sse.rs         Server-sent events parser
    │       ├── catalog.rs     Model list fetch and cache
    │       └── pricing.rs     Cost calculation
    ├── tether-tools/
    │   └── src/
    │       ├── lib.rs         Agent tool contracts and implementations
    │       ├── fs.rs          Bounded, workspace-scoped read tools (tests in fs/tests.rs)
    │       └── edit.rs        Atomic text edits kept inside a project workspace (tests in edit/tests.rs)
    ├── tether-workspace/
    │   └── src/
    │       ├── lib.rs           Project workspace: path guard, file index, search, and archives
    │       ├── path_guard.rs    Canonical workspace boundary for file tools
    │       ├── file_index.rs    Workspace-relative file index and bounded text search
    │       ├── ignore_rules.rs  Git-compatible ignore rules for the index and archives
    │       ├── archive/         Safe, bounded ZIP import and export: create.rs, extract.rs, names.rs (entry name checks)
    │       └── *_tests.rs       Test modules
    ├── tether-vcs/
    │   └── src/
    │       ├── lib.rs         Git and GitHub support
    │       ├── clone.rs       Repository cloning through gitoxide
    │       ├── repo.rs        Local repository operations
    │       ├── repo/          checkout.rs, diff.rs, helpers.rs (branch names, staging), log.rs, ops.rs (open, init, status, diff, stage), stageable.rs (files a stage-all adds)
    │       ├── push/          Smart HTTP receive-pack: mod.rs, http.rs (blocking requests, no credential in the URL), protocol.rs
    │       ├── github/        GitHub REST client (client.rs) and helpers (URL parsing, shallow clone choice)
    │       ├── oauth.rs       GitHub OAuth device flow; the token is wrapped by the secret store
    │       ├── pat.rs         GitHub personal access token check before wrapped storage
    │       ├── test_support.rs  Temporary directory and Git repository helpers for tests
    │       └── *_tests.rs, tests.rs, oauth/tests.rs, pat/tests.rs  Test modules
    ├── tether-sandbox/
    │   └── src/
    │       ├── lib.rs               WASM sandbox executor on Wasmtime
    │       ├── module_cache.rs      One process engine and an in-memory compiled module cache
    │       ├── runner.rs            Blocking WASI preview 1 command runner
    │       ├── runner_result.rs     Result classification and output pumping
    │       ├── runner_support.rs    Runner timing and host-path helpers
    │       ├── runner_validation.rs Checks for support mounts and guest settings
    │       ├── wasi_context.rs      Input contract for the WASI context and its streamed output
    │       ├── limits.rs            Per-run limits and bounded streaming output
    │       ├── output_pipe.rs       Guest stdout and stderr pipe that keeps at most the output cap in memory
    │       ├── pack.rs              Signed runtime pack manifests and streaming verification
    │       ├── pack_install.rs      Atomic pack installation and on-device integrity checks
    │       ├── pack_replace.rs      Atomic replacement of verified packs
    │       ├── pack_catalog.rs      Queries over verified installed packs
    │       ├── pack_cleanup.rs      Cleanup helpers for failed pack replacement
    │       ├── pack_io.rs           Streaming pack file hashing
    │       ├── pack_layout.rs        Staging, layout, and manifest writes for pack installs
    │       ├── test_support.rs      Shared WASI test fixtures
    │       └── *_tests.rs           Test modules (pack_tests.rs, runner_tests.rs)
    ├── tether-linux/
    │   ├── src/
    │   │   ├── lib.rs         Linux mode: process execution and Alpine root filesystem management
    │   │   ├── error.rs       LinuxError
    │   │   ├── rootfs.rs      Root filesystem manager: cache check, status, progress, install
    │   │   ├── rootfs_io.rs   Cache and file-index checks for the root filesystem
    │   │   ├── manifest.rs    Root filesystem manifest, image checks, and the fetcher trait
    │   │   ├── extract.rs     Extract and repair the root filesystem archive
    │   │   ├── proot.rs       PRoot command plan, network policy, and session kinds
    │   │   ├── runner.rs      Runs commands inside the guest and reports outcomes
    │   │   ├── shell.rs       Interactive shell over a PTY
    │   │   └── packages.rs    Base package install and setup through apk
    │   └── tests/             Integration and hardening tests (fixtures, command builder, manifest, packages, rootfs, runner, shell, archive hardening)
    ├── tether-ci/
    │   ├── src/
    │   │   ├── lib.rs         GitHub Actions workflow management, polling, and failure reports
    │   │   ├── client.rs      GitHub Actions API client for the tether.yml workflow
    │   │   ├── client/support.rs  Conversions from API runs and jobs to CI types
    │   │   ├── logs.rs        Log trimming, ANSI stripping, failure markers, pytest output
    │   │   ├── logs/support.rs    Location parsing for log failure frames
    │   │   ├── poll.rs        Run polling with an injected sleep and a terminal-state check
    │   │   └── workflow.rs    Generated workflow file and its version marker check
    │   └── tests/
    │       └── ci_tests.rs    Contract tests for the client, poller, and log parser
    ├── tether-quality/
    │   ├── data/
    │   │   ├── generic_names.txt   Generic identifier names that the slop rules flag
    │   │   ├── slop_patterns.toml  Regex and AST patterns for slop
    │   │   └── slop_words.txt      Banned words for writing and identifiers
    │   ├── tests/                  Integration tests and fixtures/ (Markdown and Rust samples)
    │   └── src/
    │       ├── lib.rs         Quality gate parsing, rules, and syntax highlighting
    │       ├── highlight.rs   Tree-sitter syntax highlighting
    │       ├── language.rs    Tree-sitter languages and language-specific rule data
    │       ├── manifest.rs    Project manifest package extraction
    │       ├── model.rs       Public data types shared by the rules
    │       ├── prose.rs       Prose checks shared with the slop-lint data files
    │       ├── registry.rs    Rule registry and batch execution
    │       ├── report.rs      Quality report construction and rendering
    │       ├── review.rs      Second-model review prompt and response parsing
    │       ├── thresholds.rs  Quality thresholds and config parsing
    │       └── rules/
    │           ├── mod.rs         Built-in rules
    │           ├── slop.rs        AI slop rules for source code
    │           ├── smell.rs       Tree and batch based smell rules
    │           ├── smell_text.rs  Text-oriented smell rules
    │           ├── tests_rules.rs Test quality rules
    │           └── util.rs        Shared helpers for heuristic rules
    ├── tether-store/
    │   ├── migrations/
    │   │   ├── 0001_init.sql            Schema: projects, sessions, messages, provider settings, model catalog, usage
    │   │   ├── 0002_message_fts.sql     FTS5 text parts; the message row stays the source of truth
    │   │   ├── 0003_checkpoints.sql     Checkpoints and their files: content before each edit batch
    │   │   ├── 0004_approvals.sql       Approvals and project permission rules
    │   │   ├── 0005_prompt_snapshot.sql Frozen prompt blocks per session
    │   │   ├── 0006_session_todos.sql   Session checklist
    │   │   ├── 0007_project_trust.sql   Project trust flags
    │   │   ├── 0008_project_network.sql Per-project Linux network permission
    │   │   ├── 0009_git_identity.sql    The user's Git commit identity
    │   │   ├── 0010_budgets.sql         Budget limits and whether session prices were known
    │   │   ├── 0011_context.sql         Context compaction records and the global threshold
    │   │   ├── 0012_mcp_servers.sql       MCP servers, header secret references, and tool settings
    │   │   └── 0013_skills_memory.sql     Skill settings, disabled skills, staged skill changes, usage, and the skills block in prompt snapshots
    │   └── src/
    │       ├── lib.rs               SQLite storage: schema, migrations, queries
    │       ├── db.rs                Connection: open, set up, migrate
    │       ├── error.rs             StoreError; the engine converts it to CoreError
    │       ├── migrations.rs        Numbered migrations and the runner
    │       ├── projects.rs          Project rows and workspace directory names
    │       ├── sessions.rs          Session and transcript writes; text is committed before its event is emitted
    │       ├── session_load.rs      Loads one saved session row
    │       ├── session_search.rs    Session list, message search, and rename
    │       ├── session_fts.rs       Exact phrase search over saved transcript text
    │       ├── session_context.rs   Session context visibility
    │       ├── session_plans.rs     Latest submitted session plan
    │       ├── checkpoints.rs       Content-addressed snapshots before an edit batch
    │       ├── approvals.rs         Stored approval answers
    │       ├── permission_rules.rs  Project-scoped permission rules
    │       ├── project_trust.rs     Project trust flags
    │       ├── project_network.rs   Per-project Linux network permission
    │       ├── prompt_snapshots.rs  Frozen prompt blocks per session
    │       ├── provider_configs.rs  Saved provider state; secret bytes stay in the platform vault
    │       ├── models.rs            Model catalog cache queries
    │       ├── todos.rs             Session checklist persistence
    │       ├── budgets.rs           Budget settings and session overrides
    │       ├── usage.rs             Usage history and budget persistence
    │       ├── context.rs           Persisted context compaction and interrupted-turn operations
    │       ├── mcp_servers.rs       MCP server, header-reference, and tool-setting queries
    │       ├── skill_settings.rs    Settings and small records for skills, memory, and vault tools
    │       └── *_tests.rs, tests.rs, checkpoints/tests.rs, session_fts/tests.rs  Test modules (schema_tests.rs covers 0001)
    ├── tether-connectors/
    │   └── src/
    │       ├── lib.rs         Network connectors used by the core
    │       ├── backend.rs     Web backend contract and the DNS-filtering reqwest backend
    │       ├── fetch.rs       Safe web fetching and model rendering
    │       ├── html.rs        HTML cleanup and readable text conversion
    │       ├── content.rs     Content type and charset helpers
    │       ├── ip.rs          Public IP address classification for SSRF protection
    │       └── hardening_tests.rs  Hardening test module
    ├── tether-mcp/
    │   └── src/
    │       ├── lib.rs         MCP client for Streamable HTTP servers
    │       ├── transport.rs   Transport contracts and the production HTTP transport
    │       ├── protocol.rs    JSON-RPC and SSE response handling
    │       ├── session.rs     MCP initialization and tool operations
    │       ├── names.rs       Stable names for tools exposed by remote servers
    │       ├── render.rs      MCP result rendering
    │       ├── tests.rs       Test module
    │       └── hardening_tests.rs  Hardening test module
    ├── tether-secrets/
    │   └── src/
    │       └── lib.rs         Secret refs, wrap and unwrap through the platform callback
    ├── tether-skills/
    │   └── src/
    │       ├── lib.rs           Parse and validate open Agent Skills documents (SKILL.md frontmatter)
    │       ├── discovery.rs     Skill discovery, safe loading, and prompt index rendering
    │       ├── discovery/file_ops.rs  Filesystem checks for skill discovery
    │       ├── index.rs         Prompt index rendering for discovered skills
    │       ├── scan.rs          Static checks on skill files before installation
    │       ├── scan_rules.rs    Pattern checks the scanner applies to each line and file
    │       ├── upkeep.rs        Pure planning for prune-only skill upkeep
    │       └── scan_tests.rs, tests.rs, upkeep_tests.rs  Test modules
    └── tether-memory/
        └── src/
            ├── lib.rs         Capped agent and user notes, a frozen snapshot, and an Obsidian-compatible vault
            ├── files.rs       Writes to the two memory files, validated before persistence (tests in files/tests.rs)
            ├── snapshot.rs    Session-scoped view of the two memory files
            └── vault/
                ├── mod.rs     Rebuildable view of Markdown notes and wikilinks in a private vault folder
                ├── tools.rs   File operations and bounded queries over the vault view
                ├── sync.rs    Copy-based sync contract with a platform shared folder
                └── tests.rs   Test modules in vault/, vault/sync/, and files/
```

Each crate has a `Cargo.toml`. Integration tests go in the crate's `tests/` folder where it has one.

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
│       │   │   ├── shell/                SettingsScreen, ProjectsScreen, and placeholder screens until feature modules exist
│       │   │   ├── updates/              AboutScreen and the update card, UpdatesController (state holder), UpdateGateway, UpdateStore, UpdateInstaller (interface), and the install status mapping
│       │   │   ├── providers/            ProvidersScreen, ProviderDetailScreen, their ViewModels and UI state, ProviderGateway (the only door to the core's ProviderSettings)
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
│       │   ├── AndroidManifest.xml   REQUEST_INSTALL_PACKAGES; Linux mode components arrive with P7
│       │   ├── kotlin/app/tether/updates/  PackageInstaller installer: stages the verified APK and shows the system confirmation
│       │   └── jniLibs/              (gen) libproot.so, libproot-loader.so per ABI (planned, P7)
│       └── play/
│           ├── kotlin/app/tether/flavor/LinuxModeStub.kt  Linux mode unavailable
│           └── kotlin/app/tether/updates/  Store link installer: opens Google Play, never installs
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
│   └── release.yml          Build, sign, and publish a GitHub Release for both flavors on every merge that changes code
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
