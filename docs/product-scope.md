# Product scope

Status: draft 0.2, 18 Nov 2026. This is the full definition of what Tether does. project-overview.md is the short version.

## 1. Problem

Coding agents such as Claude Code, Codex, OpenCode, and Qwen Code have changed how developers work, but they all assume a computer: a terminal, a filesystem, installed runtimes, and a machine that stays awake. Away from that computer, a developer can at most watch a remote session through a phone, and only if the computer is on and reachable.

Many developers do not have a capable computer with them, and some do not own one at all. Phones, meanwhile, have enough memory and network to drive an agent, since the model runs in the cloud. What is missing is the rest of the harness: a workspace, tools, a place to run code, and a UI built for a small screen.

Existing "one window for every agent" apps (AionUi, Orca, T3 Code, Pixcode and others) all wrap desktop CLIs, so they also need a desktop.

## 2. Solution

Tether puts the whole harness on the phone:

- The agent loop, tools, git, and storage run in a Rust core inside the app.
- The model is whichever provider the user configures, called directly from the phone.
- Code runs in three places, chosen automatically: a WASM sandbox in the app, a Linux environment in the app, or GitHub Actions.
- The UI follows the conventions desktop agents established, redesigned for touch and a narrow screen.

## 3. Who it's for

| Persona | Situation | What they need from Tether |
|---|---|---|
| Away-from-desk developer | Uses Claude Code or Codex daily; wants to fix a bug or review agent work while commuting | Familiar commands and modes, fast diff review, push and PR from the phone |
| Phone-first developer | Main device is a phone; may share or borrow a computer | Full workflow with no PC, low data use, honest cost display |
| Founder or tech lead | Writes less code, steers more | Plan mode, approvals from notifications, CI results in the chat |

## 4. Principles

The six principles in project-overview.md apply. Two more guide feature decisions:

1. **Every feature works on a 360 dp wide screen with one thumb.** If it cannot, it is redesigned or cut.
2. **The cheapest correct path is the default.** Cheaper models for exploration when the user allows it, the sandbox before CI, and compaction before running out of context.

## 5. Core concepts

Defined in project-overview.md. The glossary at the end of this doc lists every term.

## 6. Features

Each feature lists what it is, how it works, and why it exists. IDs (F1 and so on) are used in build-plan.md and progress-tracker.md.

### F1. Providers and models

**What.** Connect any of Anthropic, OpenAI, Google Gemini, DeepSeek, and OpenRouter with an API key. Pick a model per project, change it per session, and set a default in Settings.

**How.**
- Each provider has an adapter in the Rust core that converts Tether's internal message format to the provider's API and back, including streaming and tool calls. DeepSeek and OpenRouter use the OpenAI-compatible chat format; Anthropic, OpenAI, and Gemini use their native APIs.
- Each provider row in Settings has a key field (masked, with paste and a reveal toggle behind biometric) and a **Test connection** button that makes the cheapest possible authenticated call and reports success, a bad key, a network failure, or a region block.
- The model picker lists models from each provider's model-list endpoint, cached for 24 hours. Each entry shows context window, price per million input and output tokens where known, and capability badges: tools, images, reasoning.
- Models without tool support are shown but disabled for agent sessions, with the reason.
- Reasoning effort (low, medium, high) appears for models that support it.

**Why.** Users already pay for one or more of these. Five providers through two API shapes covers almost every model a developer wants, and OpenRouter covers the long tail.

### F2. Projects and workspaces

**What.** A project is a code repository in a workspace folder. Private storage is the default. The
`full` flavor offers a shared folder after the person grants All files access. Create one empty,
clone from GitHub, or import a zip from phone storage.

**How.**
- New projects use the app's private storage by default. Settings and onboarding show the current
  projects folder and the other local data paths. Existing projects keep their roots when the
  folder for new projects changes.
- The `full` flavor can use a shared folder after `MANAGE_EXTERNAL_STORAGE` is granted. Other apps
  and a computer can see files in that folder. The Play flavor stays private.
- Cloning uses gitoxide over HTTPS with the user's GitHub token. Shallow clone (depth 1) is the default for repos over 200 MB, with an option for full history.
- Zip import goes through the Android file picker. Export produces a zip through the share sheet.
- The project home shows the repo name, branch, last session, uncommitted change count, and default executor.
- A file index (paths, sizes, languages) is built on open and kept current with a filesystem watcher. `.gitignore` rules apply to search and agent tools.

**Why.** The workspace is the boundary for everything the agent can touch. Private storage is
fast and safe by default. The shared option is explicit and visible so the person can use files
with other apps or a computer when needed.

### F3. Sessions and transcript

**What.** A session is one conversation in a project. The transcript shows user messages, agent text, tool calls, diffs, plans, runs, and quality reports in order.

**How.**
- Sessions persist in SQLite and resume exactly where they stopped, including after the app is killed.
- Agent text streams in as it arrives. Tool calls appear as cards that update as they move from proposed to running to done.
- Long tool output collapses to the first and last 12 lines with an expand control.
- A status line under the top bar shows model, permission mode, context used (percent), and session cost.
- Sessions list per project, sorted by last activity, with search over titles and message text.
- The first user message names the session; the user can rename it.

**Why.** Desktop agents taught users to read a transcript to understand what happened. Tether keeps that model so the mental model carries over.

### F4. Agent tools

**What.** The actions the agent can take.

| Tool | What it does | Needs approval in Ask mode |
|---|---|---|
| `read_file` | Read a file or a line range | No |
| `list_dir` | List a folder | No |
| `glob` | Find files by pattern | No |
| `grep` | Search file contents with a regex | No |
| `write_file` | Create or overwrite a file | Yes |
| `edit_file` | Replace an exact string in a file | Yes |
| `multi_edit` | Several exact replacements in one file, applied together | Yes |
| `move_file` / `delete_file` | Rename or delete | Yes |
| `run` | Run a command through the executor router (F10); `background: true` starts a Linux dev server that stays until stopped | Yes |
| `process_list` / `process_logs` | Read session-owned background processes and their bounded logs | No |
| `process_stop` | Stop a session-owned background process tree | Yes |
| `preview` | Open a guarded project HTML file or a session-owned `127.0.0.1` server | No |
| `git_init` | Initialize the project's local repository | Yes |
| `git_status` / `git_diff` / `git_log` | Read repo state | No |
| `git_commit` / `git_branch` | Change local repo state | Yes |
| `github_repo_create` | Create a GitHub repository and connect it as `origin` | Always, in every mode except Bypass |
| `git_push` / `pr_create` | Change remote state | Always, in every mode except Bypass; force push asks even in Bypass |
| `ci_run` / `ci_logs` | Start a workflow run and read its results | Yes |
| `web_fetch` | Fetch a URL as text | Yes |
| `web_search` | Search through the provider's native search where offered | Yes |
| `todo_write` | Create and update the plan checklist | No |
| `ask_user` | Ask the user a question with tappable options | No |
| MCP tools | Tools from connected MCP servers (F16) | Yes |
| `skills_list` / `skill_view` | List skills and load one or one of its files (F21) | No |
| `skill_manage` | Create, patch, or delete a skill (F21) | Yes, staged as a diff |
| `memory` | Add, patch, or remove an entry in `MEMORY.md` or `USER.md` (F22) | No; shown in the transcript, and a setting can ask first |
| `session_search` | Search past sessions (F22) | No |
| `vault_search` / `vault_read` / `vault_links` / `vault_map` / `vault_orphans` | Read the vault (F23) | No |
| `vault_write` / `vault_append` | Write or append a vault note (F23) | No; shown in the transcript, and a setting can ask first |

**How.** Tools are defined once in the Rust core with JSON schemas generated from Rust types. Every tool runs through the path guard (no access outside the workspace, no following symlinks out of it) and the permission check (F5). Results return to the model in a compact format with truncation markers.

**Why.** This set matches what desktop harnesses offer, so prompts and habits transfer. Exact-string edits are the safest edit format for models: they fail loudly instead of corrupting files.

### F5. Permission modes and approvals

**What.** Five modes that decide which tool calls need the user's approval.

| Mode | Reads | File edits | `run` in sandbox | `run` in Linux mode or CI | Network tools | Push and PR |
|---|---|---|---|---|---|---|
| Plan | Auto | Blocked | Blocked | Blocked | Ask | Blocked |
| Ask | Auto | Ask | Ask | Ask | Ask | Ask |
| Accept edits | Auto | Auto | Ask | Ask | Ask | Ask |
| Auto | Auto | Auto | Auto | Ask | Ask | Ask |
| Bypass | Auto | Auto | Auto | Auto | Auto | Auto, except force push |

Background runs use the same Linux-mode approval as other host runs. The approval names the command and says it keeps running until stopped.

**How.**
- The mode chip in the composer shows the current mode; tapping it opens a picker. `/mode` does the same.
- An approval appears as a bar above the composer with the exact action: the command text, the file and diff, or the URL. Buttons: **Allow**, **Deny**, and **Always allow this** (adds a rule scoped to the project, such as "allow `pytest` in sandbox").
- When the app is in the background, approvals arrive as high-priority notifications with Allow and Deny actions. Edits show a short diff summary in the notification; the full diff opens the app.
- Bypass needs biometric confirmation to turn on, shows a red status line while on, and switches off when the session ends.
- Deny can include a reason, which goes back to the agent.

**Why.** Desktop users rely on these modes to balance speed and control. Approvals from notifications matter more on a phone, where the user is often doing something else.

### F6. Plan mode and task checklist

**What.** In Plan mode the agent investigates read-only and returns a plan. During any session the agent can keep a checklist of steps.

**How.**
- Plan mode allows reads and blocked-by-default network tools. The agent ends with a plan card: numbered steps, files it expects to touch, commands it expects to run, and open questions.
- Plan card buttons: **Run this plan** (switches to the user's chosen working mode, Accept edits by default), **Edit plan** (opens the plan as editable text), **Keep planning**.
- `todo_write` keeps a checklist pinned at the top of the transcript, collapsed to the current step.

**Why.** Reviewing a plan is cheaper than reviewing a diff, especially on a small screen.

### F7. Slash commands, mentions, custom commands

**What.** Typed shortcuts in the composer.

| Command | Action |
|---|---|
| `/new` | Start a new session in this project |
| `/clear` | Clear context but keep the session record |
| `/compact [focus]` | Summarize the context now, optionally keeping a focus |
| `/model` | Open the model picker |
| `/mode` | Open the permission mode picker |
| `/plan` | Switch to Plan mode |
| `/diff` | Open uncommitted changes |
| `/rewind` | Open the checkpoint list |
| `/commit` | Ask the agent to write a commit message and commit |
| `/push` | Push the current branch |
| `/pr` | Draft and open a pull request |
| `/run <cmd>` | Run a command yourself in the default executor |
| `/linux` | Open a Linux mode shell for this project |
| `/ci` | Show recent CI runs |
| `/review` | Second-model review of uncommitted changes (F12) |
| `/cost` | Show cost for this session and project |
| `/context` | Show what fills the context window |
| `/skills` | List skills; open one; install one |
| `/memory` | Show `MEMORY.md` and `USER.md`; edit them by hand |
| `/vault` | Open the vault; `/vault lint` lists orphan notes and unresolved links |
| `/init` | Generate a `TETHER.md` for this project |
| `/export` | Export the session as Markdown |
| `/help` | List commands |

`@` opens a file picker that inserts a file reference; the agent receives that file's content with the message. Custom commands are Markdown files in `.tether/commands/`; the filename is the command name and `$ARGUMENTS` is replaced with the typed text.

**How.** Typing `/` or `@` at the start of a word opens a filtered sheet above the keyboard. Commands also live in a menu behind the composer's plus button, so nothing depends on typing.

**Why.** Same commands as desktop harnesses where the meaning matches, so muscle memory transfers.

### F8. Project instructions

**What.** A `TETHER.md` at the repo root gives the agent standing instructions: how to run tests, conventions, things to avoid.

**How.** Loaded into the system prompt at session start. If absent, Tether reads `AGENTS.md`, then `CLAUDE.md`. `/init` asks the agent to inspect the repo and draft one, shown as a diff for approval. A global instructions file in Settings applies to all projects. Project settings live in `.tether/config.toml` (executor defaults, allowed commands, quality thresholds).

**Why.** Users of other harnesses already have `AGENTS.md` or `CLAUDE.md` files; reading them makes existing repos work on day one.

### F9. Diff review, checkpoints, rewind

**What.** See every change, accept or reject it per file or per hunk, and roll back to any earlier point.

**How.**
- Each edit batch produces a diff card in the transcript: files changed, lines added and removed, and the quality gate result. Tapping opens the diff view.
- The diff view is unified (not side by side) on compact screens, with per-hunk **Keep** and **Undo** and a per-file overflow menu. Long lines wrap; a toggle switches to horizontal scroll.
- Before each edit batch, the core stores the prior content of every touched file as content-addressed blobs. That is a checkpoint.
- `/rewind` lists checkpoints by turn. Restoring offers: files only, conversation only, or both.
- Checkpoints are independent of git and work in projects with no repo.

**Why.** Rewind makes Auto mode safe to try. Per-hunk control matters when the agent gets one part right and another wrong.

### F10. Executors and routing

**What.** Three places to run commands.

| Executor | Runs | Speed | Limits |
|---|---|---|---|
| Sandbox | WebAssembly runtimes inside the app: Python, JavaScript (QuickJS), SQLite, and tools compiled to WASI | Starts in under 100 ms | No native extensions, no threads in most runtimes, no network unless allowed per project |
| Linux mode | Real Alpine Linux userland through PRoot: shell, package manager, Node, Python, compilers | Seconds to start; slower than a desktop | `full` flavor only in v1; downloads about 10 MB base plus packages; Android-only |
| CI | GitHub Actions in the project's repo | Minutes per run | Needs a GitHub repo and Actions minutes |

**How.**
- The `run` tool takes a command and an optional executor. With no executor given, the router picks: the project default from `.tether/config.toml`, then a match table (for example `python`, `pytest` on pure-Python code, and `sqlite3` go to the sandbox; `npm install`, `cargo`, `gradle` go to Linux mode if installed, else CI).
- `run` accepts `background: true` only with Linux mode. It returns a short process id, initial output, and a loopback URL when port discovery finds one. The process, its last 256 KiB of output, and its detected port belong to the session and stop when the session closes. Sandbox and CI reject background runs with `background runs need Linux mode`.
- The sandbox gives each run a fresh instance with the workspace mounted read-write at `/workspace`, a memory cap (512 MB default), a time limit (60 s default), and no secrets in its environment. Runtime packs (Python, JavaScript, SQLite) download on first use, are verified against a signed manifest, and are precompiled on the device.
- Linux mode is opt-in per device. Setup downloads the Alpine mini root filesystem, verifies its checksum against Tether's signed `manifest.json` and detached `manifest.json.sig`, and installs base packages. Each project is bind-mounted at `/workspace`. Network access is a per-project toggle, off by default, and asks on first need.
- CI uses a workflow file Tether adds to the repo (`.github/workflows/tether.yml`, shown as a diff first) that accepts a command input. The agent pushes to a work branch, dispatches the workflow, polls status, then reads logs. Failures are parsed into file and line references where the format allows.
- A run card in the transcript shows the executor, command, live output, exit code, and duration. A **Run elsewhere** button reruns on another executor.

**Why.** The sandbox keeps the agent loop fast and safe. Linux mode handles what WASM cannot, without a PC. CI is the fallback that can run anything, including Android builds.

### F11. Git and GitHub

**What.** Initialize a local repository, create a GitHub repository, branch, commit, push, and open pull requests.

**How.**
- GitHub sign-in uses the OAuth device flow: Tether shows a code, the user approves it in the browser, and the token is stored wrapped by the Keystore. A fine-grained personal access token can be pasted instead.
- Sign-in accepts classic and fine-grained tokens. Tether persists the token kind, classic scopes, login, and whether `/user/repos` is readable, so GitHub status survives a restart.
- `git_init` works in a project with existing files. `github_repo_create` creates the repository, adds its clone URL as `origin`, and saves the remote so the existing push and pull request tools can continue the flow.
- The agent works on a branch named `tether/<short-task-slug>` by default, never directly on the default branch unless the project setting allows it.
- `git_push` to the default branch, force push, and remote branch deletion always ask, even in Bypass (force push) or Auto (the others).
- Commit messages and PR descriptions are drafted by the agent, run through `slop-lint` rules, and shown for editing before use.
- The PR card shows the PR link, CI status checks, and review comments; `/pr` refreshes it.

**Why.** GitHub is where phone-made work becomes real. Guards on remote changes protect shared branches.

### F12. Quality gate: code smells and AI slop

**What.** Automatic checks on every edit batch, plus an on-demand review by a second model.

**How.**
- The gate parses changed files with tree-sitter and runs three rule sets on the changed regions only:
  - **Code smells:** function length, nesting depth, parameter count, file length, duplicate blocks, dead code, commented-out code, magic numbers, swallowed errors, debug prints left in, TODO without an issue reference.
  - **AI slop in code:** comments that restate the next line, placeholder bodies (`pass`, `TODO: implement`, `unimplemented!()` in non-test code), imports of packages not in the manifest, broad try/catch around code that cannot throw, unused helper functions added in the same batch, generic names (`data2`, `handleStuff`, `utils2`), banned words in identifiers and strings, emoji in logs.
  - **AI slop in tests:** assertions that cannot fail (`assert true`, `expect(x).toBe(x)`), tests with no assertions, deleted or skipped tests in a batch that also changes the code under test, mocks of the unit under test, snapshot updates with no code change explaining them.
- Each finding has a severity: info, warn, or block. Thresholds come from `.tether/config.toml`, with defaults from code-standards.md.
- The report appears on the diff card. In Auto mode, block-level findings stop the commit and go back to the agent as a tool result: "Fix these before committing."
- Language linters (for example a Python linter in the sandbox) run when available and their output joins the report.
- `/review` sends the uncommitted diff, the task, and the quality report to a second model (set in Settings, can differ from the main model) with a reviewer prompt. Its findings appear as a review card with file and line links.
- Commit messages, PR text, and `TETHER.md` drafts go through the writing slop rules in project-overview.md.

**Why.** Agents produce code that runs but reads badly, and they sometimes delete tests to make a suite pass. Catching this before commit is cheaper than catching it in review, and reviewing on a phone is already harder.

### F13. Long runs and background work

**What.** A turn keeps running when the user leaves the app or turns off the screen.

**How.**
- When a turn starts, the app starts a foreground service with a persistent notification showing the current step and an **Interrupt** action.
- If Android kills the process anyway, the core has written the session state after every event. On next launch, the session shows "Interrupted while running `<step>`" with **Resume** and **Discard**. Resume replays the last tool result to the model.
- Network loss pauses the turn and resumes when the connection returns, with backoff. Partial streamed text is kept.
- First-run guidance asks the user to exempt Tether from battery optimization, with OEM-specific instructions for Samsung, Xiaomi, Oppo, Vivo, and Huawei.

**Why.** Agent turns can take many minutes. On a phone, the user will switch apps; losing work is the fastest way to lose trust.

### F14. Context management

**What.** Keep long sessions within the model's context window, and pay less for repeated context.

**How.**
- A context meter in the status line shows percent used. `/context` breaks it down: system prompt, instructions, tool definitions, files, conversation.
- At 80% (configurable), Tether compacts: a summary of the session so far replaces older messages, with a marker in the transcript. `/compact` does this on demand.
- Prompt caching is used for Anthropic, OpenAI, Gemini, and DeepSeek where the API supports it. The system prompt, tool schemas, and instructions are ordered first so they cache.
- Large tool results are truncated with a marker telling the model how to read more.
- Memory (F22) and the vault slice (F23) enter the prompt once, at session start, after the instructions and before the conversation. Mid-session writes do not change the prompt, so the cached prefix stays valid. The skill index (F21) is also part of that prefix.

**Why.** Running out of context mid-task is a common failure in long agent runs. Caching cuts cost noticeably for long sessions.

### F15. Web tools

**What.** Fetch a page and search the web.

**How.** `web_fetch` downloads a URL from the core, strips it to readable text, and truncates. `web_search` uses the selected provider's built-in search where it exists (Anthropic web search, OpenAI web search, Gemini grounding, OpenRouter's online models). If the provider has none, the tool is hidden.

**Why.** Agents need current documentation. Using provider search avoids a separate search API key.

### F16. MCP servers

**What.** Add tools from Model Context Protocol servers.

**How.** v1 supports remote servers over Streamable HTTP, added by URL with optional auth header or OAuth. In Linux mode, stdio servers can run inside the Linux environment. Each server's tools appear in Settings with per-tool enable toggles and default approval.

**Why.** MCP is how users already extend desktop harnesses. Remote servers need no PC.

### F17. Cost and usage

**What.** Know what each turn, session, and project cost.

**How.** Token counts and cost come from provider usage fields; prices come from the model catalog. The status line shows session cost. `/cost` shows a breakdown with a 30-day chart per project and per provider. Optional budgets per session and per day warn at 80% and pause the agent at 100% until the user continues.

**Why.** Bring-your-own-key users carry the cost directly. Budgets stop a stuck loop from running up a bill.

### F18. Input: voice, images, share sheet

**What.** Talk instead of type, attach screenshots, and send text or files into Tether from other apps.

**How.** The composer has a microphone button using the Android speech recognizer, with the transcript editable before sending. Images attach from camera, gallery, or the share sheet, for models with image input. Sharing text or a file into Tether opens a project picker and starts a session with that content.

**Why.** Typing long prompts on a phone is slow. Screenshots of an error are often the fastest way to describe it.

### F19. Settings and security

**What.** Control keys, defaults, and safety.

**How.** Settings groups: Providers, Models and defaults, Permissions, Executors (sandbox limits, Linux mode, CI), GitHub, MCP servers, Skills and memory (skill approval, memory write approval, vault folder sync, size caps), Quality gate, Budgets, Appearance (system, light, dark), Privacy (screen capture blocking, biometric lock), Data (export, delete project, clear caches), About. Keys are viewable only after biometric check. App lock with biometric is optional.

**Why.** One place to see and change every rule the agent follows.

### F20. Onboarding

**What.** Get from install to a first useful session.

**How.** Three screens: what Tether is (one sentence and one image); add a provider key (any one is enough, skip allowed); open a project (clone from GitHub, new, or import zip). Then a sample prompt suggestion based on the repo's language. Battery optimization guidance appears before the first long run, not at install.

**Why.** S1 targets a first PR in under 15 minutes. Each step must be skippable and returnable.

### F21. Skills

**What.** Reusable instructions the agent loads when a task needs them, and writes down when it learns something worth repeating. A skill is a folder with a `SKILL.md`.

**How.**
- Format: the open Agent Skills standard. `name` (lowercase, hyphens, up to 64 characters, equal to the folder name) and `description` (up to 1,024 characters) are required. Skills written for other agents load unchanged.
- Locations: project (`.tether/skills/`, `.agents/skills/`, after the user trusts the project), user (app storage), and bundled. A project skill beats a user skill beats a bundled one.
- The system prompt holds only an index of names and descriptions. The agent calls `skill_view` to load a skill and again to load one of its reference files. The index has a size budget, so a hundred skills cost about the same as ten.
- The agent creates and improves skills with `skill_manage`, preferring small patches over rewrites. It is told to save a skill after a multi-step workflow worth repeating, after working around an error, and after the user corrects it. Content rule: lessons, not logs.
- Agent-written skills are staged and shown as a diff for approval. A setting turns this off.
- Install from a GitHub URL or a zip goes through quarantine: Tether shows the files and the result of a static scan, and installs only on approval. Skill scripts run only inside the executors, under the permission mode.
- A prune-only job marks unused skills stale after 14 days and archives them after 30. It never deletes and makes no model calls.
- `/skills` lists and opens skills. The user can also edit a skill in the file viewer.

**Why.** Repeat work should get cheaper and better. Loading skills on demand keeps the prompt small, and the open format means the user can bring skills from other tools. Approval by default follows "show before doing".

### F22. Memory

**What.** Notes that carry across sessions, plus search over every past session.

**How.**
- `MEMORY.md` holds the agent's notes (up to 2,200 characters). `USER.md` holds the user's profile and preferences (up to 1,375 characters).
- The `memory` tool adds, patches, and removes entries. A write over the cap fails, and the agent trims in the same turn. Writes are scanned for prompt injection, credentials, and invisible characters.
- The files load once at session start as a frozen snapshot. Writes land on disk at once and take effect next session. This keeps the cached prompt stable.
- `session_search` finds past messages with SQLite full-text search and returns the messages themselves. No model call is made.
- `/memory` shows both files and lets the user edit them. Everything is plain text on the device.

**Why.** Users should not repeat their preferences, and the agent should not rediscover the same facts. Small caps force it to keep only what matters. A frozen snapshot is cheaper on providers that cache prompts. Search covers what the caps cannot hold, at no token cost.

### F23. Vault

**What.** A long-term knowledge folder in Obsidian's format: plain Markdown notes with properties and `[[wikilinks]]`, plus a map of how the notes connect.

**How.**
- Each note has frontmatter with a required `description`, and optional `type`, `tags`, `aliases`, `created`, and `updated`. Links use `[[Note]]`, `[[Note#Heading]]`, and `[[Note|alias]]`.
- `index.md` lists every note by category in one line each. `log.md` is an append-only record; a correction is a new entry. Map notes (`type: moc`) group the links for one area, such as a project.
- Tether indexes notes and links into SQLite. The index is rebuilt from the files, so the folder is the source of truth.
- Tools: `vault_search`, `vault_read`, `vault_write`, `vault_append`, `vault_links` (outlinks and backlinks), `vault_orphans`, and `vault_map` (the neighborhood of a note within a token budget).
- At session start a small slice goes into the prompt: the top of `index.md`, the active project's map note, and recent log entries. Full notes are fetched on demand.
- Every write is checked: a `description` must exist, links to missing notes are recorded as unresolved, and size is capped. `/vault lint` lists notes that nothing links to.
- The Vault screen shows notes, backlinks, and a list-style map. There is no force-directed graph.
- The vault lives in app storage. The user can export or import it as a zip, or pick a shared folder to sync with. Sync copies changed files both ways, so Obsidian for Android can open the same notes. See architecture.md section 14 for why the core cannot use the shared folder directly.

**Why.** Two capped files cannot hold project history, decisions, or conventions. A linked folder of notes can, and because the format is plain Markdown the user owns it, can read it, and can open it in a tool they already use. The link map gives the agent a bounded way in, so it reads three relevant notes instead of searching the whole vault.

## 7. Integrations

Tether has no server, so every integration runs from the phone with credentials the user supplies.

| Integration | Release | How it works | What the agent can do |
|---|---|---|---|
| GitHub | v1 | OAuth device flow or personal access token; REST and GraphQL APIs; git over HTTPS | Clone, create repositories, push, PRs, Actions runs and logs, issues read |
| Telegram | v1.1 | User creates a bot with BotFather and pastes its token; the foreground service long-polls the Bot API while Tether is running | Receive approval requests and run status; send tasks to a project by message; approve or deny with inline buttons |
| Discord | v1.2 | User creates a bot application and pastes its token; the service holds a gateway connection while running | Same as Telegram, in a DM or a chosen channel |
| Google Workspace | v1.3 | Google sign-in with scopes the user grants per feature | Drive: import and export files and folders to a workspace. Sheets: read ranges as data, write results. Docs: read as Markdown, write reports. Optional Gmail and Calendar later |

All connectors use one interface in the core (`Connector`) that can provide tools to the agent and events to the app. GitHub is the first implementation, so the interface is proven before the others arrive. Chat connectors only work while the phone is online and the service is running; this is stated in their setup screen.

## 8. Out of scope

| Item | Reason | Revisit |
|---|---|---|
| iOS | Android first; core is ready for Swift | v2 |
| Web or desktop companion, PC pairing | Breaks the phone-only principle | Not planned |
| On-device model inference | Phones in the target range cannot run useful coding models fast enough | After v2, if hardware allows |
| Sub-agents | Needs context and UI design for nested work | v1.1 |
| GitLab, Bitbucket | Smaller share of the audience | v1.x, by demand |
| Shared or team sessions | Needs a server | Not planned |
| Building Android APKs on the phone | Too slow and large on device; CI handles it | Through CI only |
| Docker | Not available on Android | Not planned |
| Plugin marketplace | MCP covers extension in v1 | v2 |
| Skills hub with remote browsing | Install from a URL or zip covers v1 | v1.x, by demand |
| External user-modeling service | Needs a server and costs the user tokens | Not planned |
| Force-directed vault graph view | The list-style map covers navigation | By demand |

## 9. Roadmap

| Release | Content |
|---|---|
| v1.0 | Everything in section 6, Android, `full` and `play` flavors |
| v1.1 | Telegram connector; sub-agents for read-only exploration |
| v1.2 | Discord connector; scheduled tasks (run a prompt on a timer while charging) |
| v1.3 | Google Workspace connector (Drive, Sheets, Docs) |
| v1.4 | GitLab; tablet two-pane polish |
| v2.0 | iOS app on the same core: sandbox in interpreter mode, no Linux mode, CI as the heavy executor |

## 10. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Google Play rejects Linux mode under its policy on downloaded executable code | High | Medium | `full` flavor through GitHub Releases carries Linux mode; `play` flavor ships without it unless review approves |
| A future Android release blocks the PRoot technique | Medium | High for Linux mode users | Sandbox and CI cover most needs; test every Android beta; Linux mode is labeled best-effort |
| OEM battery managers kill long runs | High | Medium | Foreground service, state saved per event, resume flow, OEM guidance |
| Prompt injection from repo files or web pages | Medium | High | Approvals for network and remote actions; secrets never in executor environments; push guards |
| Users run up provider bills | Medium | High | Live cost display, budgets, pause at limit |
| WASM runtimes cannot run common tooling | Medium | Medium | Router sends those commands to Linux mode or CI with a stated reason |
| Diff review on a small screen is error-prone | Medium | Medium | Plan mode, per-hunk control, quality gate, `/review` |
| Provider API changes break adapters | Medium | Medium | Adapter contract tests against recorded fixtures; nightly live smoke tests |
| GitHub Actions minutes cost money on private repos | Medium | Low | Show estimated minutes per run; prefer sandbox and Linux mode |
| Name conflict with an existing "Tether" product | Medium | Medium | Trademark check before public release (Q6) |
| Memory, skills, or vault notes carry injected instructions or leaked secrets | Medium | High | Writes are scanned; prompt blocks are labeled as untrusted data; skill install uses quarantine and approval; the S7 redaction test covers memory and vault files |
| The agent fills memory with noise or wrong facts | Medium | Medium | Hard size caps; patch over add; the user can read and edit every file; search over the vault instead of loading it |

## 11. Open questions

| ID | Question | Default until decided |
|---|---|---|
| Q1 | Submit the `play` flavor with Linux mode for policy review, or ship it without? | Ship without in `play` |
| Q2 | Allow the agent to work on the default branch when the repo has no other branches? | Ask once per project |
| Q3 | Offer opt-in anonymous crash reports, or local export only? | Local export only |
| Q4 | Which Alpine packages come preinstalled in Linux mode? | `bash git nodejs npm python3 py3-pip build-base` |
| Q5 | Price model: free and open source, paid app, or free with paid extras? | Undecided; no paywall code in v1 |
| Q6 | Is "Tether" clear for trademark in software and on Google Play? | Working name |
| Q7 | Telegram and Discord: user-created bots only, or an optional hosted relay later? | User-created bots only |
| Q8 | Which Google scopes ship in v1.3? | Drive file scope and Sheets only |
| Q9 | Should the vault live in shared storage so Obsidian can open it without sync? | App storage plus zip export and folder sync |
| Q10 | Should agent-written skills and memory writes need approval by default? | Skills: yes. Memory and vault: no, shown in the transcript |

## 12. Benchmarks

### Task suite

The Tether task suite is 30 tasks in `bench/tasks/`, each with a starting repo, a prompt, and an automated check:

- 10 Python tasks (bug fixes, small features, tests) solvable in the sandbox
- 8 JavaScript or TypeScript tasks, half needing `npm install` (Linux mode or CI)
- 6 tasks in compiled languages (Rust, Go, Kotlin) that need Linux mode or CI
- 6 repo chores: write a README section, fix a CI workflow, update dependencies

Measured per task and model: pass or fail, wall time, tokens, cost, number of approvals, quality gate findings. Each release runs the suite with at least two models. The reference score is the same model on a desktop harness running the same tasks; target S2 is 85% of that score.

### Performance

| Metric | Target | Where measured |
|---|---|---|
| Cold start to project list | p95 under 1.5 s | Macrobenchmark on reference device |
| Transcript scroll | No frames over 16 ms in a 500-message session | Macrobenchmark |
| Sandbox cold start | p95 under 100 ms | `core/benches/sandbox.rs` |
| Python hello world in sandbox | p95 under 400 ms | Same |
| Grep over 10,000 files | p95 under 500 ms | `core/benches/search.rs` |
| Added latency to first token | p95 under 150 ms | Debug trace |
| Battery during an active session | Under 8% per hour on the reference device, screen off | Manual test protocol in development.md |
| App memory, excluding Linux mode | Under 300 MB | Android Studio profiler |

## 13. Glossary

| Term | Meaning |
|---|---|
| Adapter | Core module that translates between Tether's message format and one provider's API |
| AI slop | Output that looks plausible but is padded, generic, or hollow: filler words, comments restating code, tests that cannot fail |
| Approval | A user decision to allow or deny one tool call |
| BYOK | Bring your own key: the user supplies provider API keys |
| Checkpoint | Stored prior content of files touched by an edit batch |
| Code smell | A structural sign that code will be hard to change, such as deep nesting or long functions |
| Compaction | Replacing older context with a summary to free space |
| MOC | Map of content: a vault note that only groups links to related notes |
| Connector | Integration with an outside service that adds tools or events |
| Edit batch | All edits the agent makes between two model responses |
| Executor | Sandbox, Linux mode, or CI |
| `full` / `play` | The two Android build flavors |
| Hunk | One contiguous block of changes in a diff |
| Linux mode | Alpine Linux userland run through PRoot inside the app |
| MCP | Model Context Protocol, a standard for giving agents extra tools |
| Path guard | Core check that keeps tools inside the workspace |
| PRoot | A user-space tool that runs a Linux root filesystem without root access |
| Quality gate | The automated code smell and AI slop checks |
| Router | Logic that chooses an executor for a command |
| Skill | A folder with a `SKILL.md` that the agent loads on demand |
| Runtime pack | A downloadable, signed bundle of WASM runtimes for the sandbox |
| Sandbox | The WebAssembly executor inside the app |
| Session | One conversation within a project |
| Turn | One user message and the agent's full response to it |
| UniFFI | Tool that generates Kotlin and Swift bindings for Rust code |
| Vault | A folder of Markdown notes in Obsidian's format, used as long-term memory |
| Workspace | A project's private folder on the phone |
