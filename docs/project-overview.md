# Project overview

Status: draft 0.2, 18 Nov 2026. Owner: Akandwanaho Blair.

## What Tether is

Tether is a coding agent harness that runs entirely on an Android phone. It works like Claude Code, Codex, OpenCode, or Qwen Code: you describe a task, the agent reads and edits files, runs commands, and shows you diffs to approve. The difference is that there is no PC anywhere in the loop. The project lives in a workspace on the phone, code runs in an on-phone sandbox or an on-phone Linux environment, and heavy builds go to GitHub Actions.

Tether brings your own keys for five model providers: Anthropic, OpenAI, Google Gemini, DeepSeek, and OpenRouter. There is no Tether server. The app talks directly to the providers you configure and to GitHub.

## Goals

1. Let a developer do real coding work (fix a bug, add a feature, write tests, open a pull request) from a phone alone.
2. Match the working style of desktop coding agents closely enough that their users feel at home: sessions, permission modes, plan mode, slash commands, diffs, checkpoints, cost tracking.
3. Run code on the phone fast enough to keep the agent loop tight, and fall back to CI only when the phone cannot do the job.
4. Keep the user in control of cost, of what runs, and of what gets pushed.
5. Catch code smells and AI slop before they reach a commit.

## Success criteria

| # | Criterion | Target | How we measure |
|---|---|---|---|
| S1 | Time from install to first pull request opened from the phone | Under 15 minutes for a returning GitHub user | Scripted usability test, 5 participants |
| S2 | Task success on the Tether task suite | At least 85% of the success rate the same model reaches in a desktop harness | Benchmark run per release (see product-scope.md) |
| S3 | Long runs survive backgrounding | 99% of 30-minute runs finish with the screen off | Automated device test on the reference device |
| S4 | Sandbox cold start | p95 under 100 ms on the reference device | Benchmark in `core/benches` |
| S5 | Added latency to first streamed token | p95 under 150 ms over the raw provider latency | Trace timestamps in debug builds |
| S6 | Crash-free sessions | At least 99.5% | Local crash log export from beta testers |
| S7 | API keys in logs, transcripts, or exports | Zero | Automated redaction test in CI |
| S8 | Blocking quality findings that reach a commit in Auto mode | Zero | Quality gate test fixtures |

Reference device: an Android 10+ phone with 4 GB RAM and a mid-range 2020-era 8-core SoC.

## Audience

Primary: developers who already use a desktop coding agent and want to keep working away from their computer, or who do not own a capable computer at all.

Secondary: students and early-career developers in places where a phone is the main computing device; technical founders who review and nudge work while away from a desk.

Not the audience in v1: non-developers who want an app built without seeing code, and teams that need shared sessions.

## Principles

1. **Phone-only, for real.** No feature may require a PC. GitHub Actions is allowed because it needs only a GitHub account.
2. **Show before doing.** The agent states what it will change or run before it happens, at the level of detail the permission mode asks for.
3. **Fast loop first.** Prefer the fastest place that can do the job: sandbox, then Linux mode, then CI.
4. **Bring your own keys, keep your own data.** No Tether account and no Tether server. Keys stay in the Android Keystore.
5. **Familiar, then mobile.** Copy what desktop harnesses got right, then redesign what does not fit a 6-inch screen.
6. **Honest output.** Tell the user what failed, what it cost, and what the agent is unsure of. No filler.

## Core concepts

| Concept | Meaning |
|---|---|
| Project | A code repository stored in a workspace folder on the phone, usually linked to a GitHub repo |
| Workspace | The folder that holds one project's files; the only place agent tools can touch |
| Session | One conversation with the agent inside a project, with its own history, checkpoints, and cost |
| Turn | One user message and everything the agent does in response until it stops |
| Tool call | One action the agent asks to take: read a file, edit, run a command, push |
| Permission mode | The rule for which tool calls need approval: Plan, Ask, Accept edits, Auto, Bypass |
| Checkpoint | A snapshot of changed files taken before each batch of edits, used by rewind |
| Executor | Where a command runs: Sandbox (WASM), Linux mode (PRoot), or CI (GitHub Actions) |
| Quality gate | Automated checks for code smells and AI slop that run on each edit batch |
| Provider | A model API account: Anthropic, OpenAI, Gemini, DeepSeek, or OpenRouter |
| Connector | An outside service the agent can use: GitHub in v1; Telegram, Discord, and Google Workspace later |
| Skill | A folder with a `SKILL.md` that the agent loads when a task needs it, and can write itself |
| Memory | Two small capped notes files, `MEMORY.md` and `USER.md`, plus search over past sessions |
| Vault | A folder of Markdown notes in Obsidian's format that holds long-term knowledge, with a link map |

## Feature list (v1)

- Five model providers with streaming, tool use, a model picker, and per-project defaults
- Projects: create empty, clone from GitHub, or import a zip
- Agent tools: read, list, glob, grep, write, edit, multi-edit, move, delete, run, git, web fetch, web search, plan checklist, ask user
- Five permission modes with approvals in the app and from notifications
- Plan mode, slash commands, file @-mentions, custom commands, `TETHER.md` project instructions
- Diff review per file and per hunk, checkpoints, and rewind
- Three executors: WASM sandbox, Linux mode, GitHub Actions, with automatic routing
- Git: branch, commit, push, pull request, with push guards
- Quality gate for code smells and AI slop, plus a `/review` second-model review
- Long runs in a foreground service, context compaction, prompt caching
- Remote MCP servers as extra tools
- Skills in the open `SKILL.md` format, loaded on demand, that the agent can create and improve with approval
- Memory: capped agent and user notes, frozen into each session, plus full-text search over past sessions
- An Obsidian-compatible vault of linked notes with backlinks and a map the agent can walk
- Token and cost tracking per turn, session, and project
- Voice input and image attachments for models that accept images

## v1 scope

Everything in the feature list above, on Android 10 and later, in two build flavors:

- `full`: all features including Linux mode, distributed as a signed APK through GitHub Releases.
- `play`: the Google Play build. Linux mode ships here only if Play policy review allows it (open question Q1).

## Out of scope for v1

iOS (planned for v2); a web or desktop companion; pairing with a PC; on-device model inference; Telegram, Discord, and Google Workspace connectors (v1.1 to v1.3); sub-agents (v1.1); GitLab and Bitbucket; team or shared sessions; a Tether server or Tether accounts; building Android APKs on the phone; Docker.

## Locked technical decisions

| ID | Decision |
|---|---|
| D1 | Rust core shared across platforms, compiled to a native library |
| D2 | Android UI in Kotlin with Jetpack Compose; iOS UI later in Swift and SwiftUI over the same core |
| D3 | Rust to Kotlin bindings generated with UniFFI |
| D4 | v1 providers: Anthropic, OpenAI, Google Gemini, DeepSeek, OpenRouter |
| D5 | Three executors: WASM sandbox (Wasmtime) as default, Linux mode (PRoot with Alpine) in v1, GitHub Actions for heavy work |
| D6 | No Tether server; bring your own keys; all network calls come from the Rust core |
| D7 | Git through gitoxide in Rust; no git binary and no Termux |
| D8 | Local storage in SQLite; secrets wrapped with an Android Keystore key |
| D9 | Android 10 (API 29) minimum; arm64-v8a and x86_64 ABIs |
| D10 | Project instructions in `TETHER.md`; `AGENTS.md` and `CLAUDE.md` read when present |
| D13 | Android application ID `io.github.khanblair.tether` (play) and `.full` suffix (full) |
| D16 | Skills, memory, and the vault ship in v1. Skills follow the Agent Skills standard; the vault follows Obsidian's Markdown format and lives in app storage with zip and folder sync |

The decisions log in progress-tracker.md records why each was made.

## Map of docs

See [README.md](README.md) for the table of every doc and the question it answers.

## Reading order for AI agents

An AI agent working on this repo reads, in order:

1. progress-tracker.md, to find the current phase and open tasks
2. this file
3. build-plan.md, the section for the current phase only
4. architecture.md, the sections for the modules the task touches
5. project-structure.md, to find where files go
6. code-standards.md, all of it
7. For UI tasks: ui-rules.md, ui-tokens.md, ui-registry.md
8. library-docs.md before adding or using any library

At the end of a session the agent updates progress-tracker.md: task status, session log, file changes log, and doc changes log.

## Writing style rules

These apply to docs, UI copy, commit messages, pull request text, and the agent's own system prompt.

1. Write for a busy developer. Lead with the answer, then the detail.
2. Use short sentences and common words. Prefer the specific word to the impressive one.
3. Sentence case for headings, buttons, and labels.
4. State numbers with units: "p95 under 100 ms", not a vague adjective.
5. No exclamation marks in docs or UI copy.
6. One idea per paragraph. Lists only when the items are parallel.
7. Name the thing. "The quality gate blocks the commit", not "this ensures quality".

### AI slop words to avoid

Do not use these words in docs, UI copy, or generated commit and pull request text. `slop-lint` flags them. The lists below are skipped by the linter; the source of truth is `core/crates/tether-quality/data/slop_words.txt`.

<!-- slop-lint:off -->

delve, seamless, seamlessly, robust, leverage, utilize, unlock, empower, elevate, supercharge, streamline, cutting-edge, state-of-the-art, game-changer, game-changing, revolutionize, transformative, next-generation, world-class, best-in-class, holistic, synergy, tapestry, testament, realm, landscape (for anything but layout orientation), embark, journey (for anything but a user flow), effortless, intuitive, powerful, comprehensive, crucial, pivotal, vital, meticulous, foster, bolster, showcase, navigate (for anything but app navigation), "in today's", "it's worth noting", "it's important to note", "let's dive in", "rest assured", "plays a key role".

The word "harness" is allowed: it names the product category.

<!-- slop-lint:on -->

### AI slop patterns to avoid

<!-- slop-lint:off -->

| Pattern | Example | Write instead |
|---|---|---|
| Restating the question | "Great question. You want to know how..." | Start with the answer |
| Empty closing summary | "In summary, Tether is a powerful tool..." | End when the content ends |
| "Not just X, but Y" | "Not just an app, but a workspace" | Say what it is |
| Forced groups of three | "fast, secure, and reliable" | List only the real properties |
| Vague intensifiers | very, really, truly, incredibly | A number or nothing |
| Hedge stacking | "may potentially help to possibly" | One hedge, or none |
| Em dash chains | Clause, then a dash, then a dash | Two sentences |
| Emoji as bullets or decoration | Rocket and sparkle icons in lists | Plain text |
| Condescending "simply" or "just" | "Simply configure your key" | "Add your key in Settings" |
| Fake precision | "Improves speed by 47%" with no source | Measured numbers with a source, or none |

<!-- slop-lint:on -->
