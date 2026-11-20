# Architecture

Status: draft 0.1, 18 Nov 2026.

## 1. System diagram

```
┌──────────────────────────── Android device ─────────────────────────────┐
│                                                                          │
│  ┌──────────────── Tether app process ───────────────────────────────┐   │
│  │                                                                    │   │
│  │  Kotlin / Jetpack Compose                                          │   │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────────────┐   │   │
│  │  │ feature-* │ │designsys. │ │  service  │ │ platform adapters │   │   │
│  │  │ screens + │ │ tokens +  │ │ foreground│ │ Keystore, speech, │   │   │
│  │  │ViewModels │ │components │ │ + notifs  │ │ files, biometric  │   │   │
│  │  └─────┬─────┘ └───────────┘ └─────┬─────┘ └─────────┬─────────┘   │   │
│  │        │       core-bridge (UniFFI generated + Flow wrappers)      │   │
│  │  ══════╪═══════════════════════════╪═════════════════╪═══════════  │   │
│  │        ▼             libtether_core.so (Rust, tokio)  ▲             │   │
│  │  ┌────────────────────────────────────────────────────┴─────────┐  │   │
│  │  │ ffi → engine (sessions, turn loop, router, permissions)      │  │   │
│  │  │   providers   tools   workspace   vcs   quality   store      │  │   │
│  │  │   sandbox (Wasmtime)   linux (PRoot runner)   ci   mcp       │  │   │
│  │  │   skills   memory (notes, vault)   connectors   secrets      │  │   │
│  │  └──────┬───────────────────────┬───────────────────┬───────────┘  │   │
│  └─────────┼───────────────────────┼───────────────────┼──────────────┘   │
│            │ in-process WASM       │ child processes   │ HTTPS            │
│            ▼                       ▼                   │                  │
│   sandbox instances        libproot.so → Alpine rootfs │                  │
│   (/workspace mounted)     (/workspace bind-mounted)   │                  │
│                                                        │                  │
│   App private storage: workspaces/, db/, blobs/, memory/, skills/, ...    │
└────────────────────────────────────────────────────────┼──────────────────┘
                                                         ▼
          Anthropic · OpenAI · Gemini · DeepSeek · OpenRouter · GitHub (API, git, Actions)
          MCP servers · Tether release manifests (static files on GitHub Releases)
```

There is no Tether backend. Every outbound call leaves from the Rust core.

## 2. Processes and threads

| Unit | What runs there | Lifetime |
|---|---|---|
| Main app process | Compose UI, ViewModels, the Rust core, sandbox instances | Android-managed; kept alive during runs by the foreground service |
| `AgentService` (same process) | Foreground service of type `dataSync`/`specialUse` that holds the process during turns, owns the run notification | Starts with a turn, stops 30 s after the last turn ends |
| Tokio runtime | One multi-thread runtime inside the core, worker count = min(4, CPU cores) | Created on first core call, lives with the process |
| Blocking pool | File I/O, git operations, tree-sitter parsing, Wasmtime compilation | Tokio blocking threads, capped at 8 |
| Sandbox instance | One Wasmtime store per `run`, on a blocking thread, with epoch-based interruption | One command |
| Linux mode child | `libproot.so` process launched from `nativeLibraryDir`, then the command inside Alpine | One command or one interactive shell |

Why one process: the core holds session state in memory and SQLite; splitting it across processes adds IPC for no benefit. Crash isolation for user code comes from Wasmtime (traps do not crash the host) and from Linux mode running in child processes.

## 3. Modules and ownership

### Rust core (`core/crates/`)

| Crate | Owns | Depends on |
|---|---|---|
| `tether-types` | Shared types: messages, tool calls, events, IDs, errors | none |
| `tether-ffi` | The UniFFI surface: exported functions, callback interfaces, type mapping | `tether-engine` |
| `tether-engine` | Sessions, the turn loop, permission checks, executor router, compaction, checkpoints | most crates below |
| `tether-providers` | One adapter per provider, streaming parsers, model catalog, usage and price math | `tether-types` |
| `tether-tools` | Tool definitions, JSON schemas, argument validation, result formatting | `tether-workspace`, `tether-vcs` |
| `tether-workspace` | PathGuard and a basic file index, glob, and text search; full ignore semantics, file ops, and watcher follow | none |
| `tether-vcs` | Git through gitoxide: clone, status, diff, commit, branch, push; P4-01 adds the GitHub device-flow core; P4-02 validates personal access tokens through `/user` and reports available scopes | `tether-types`, `tether-secrets` |
| `tether-sandbox` | Wasmtime engine, runtime packs, WASI setup, limits; P5-01 adds a process engine and in-memory module cache; P5-04 adds a fixed guest environment and output stream contract, with the actual WASI mount still pending; P5-05 adds the output cap and epoch setting, with store wiring pending | `tether-types` |
| `tether-linux` | Root filesystem manager, PRoot command builder, process I/O | `tether-types` |
| `tether-ci` | GitHub Actions: workflow file, dispatch, polling, log fetch, failure parsing | `tether-connectors` |
| `tether-quality` | Tree-sitter parsing, smell rules, slop rules, report building | `tether-types` |
| `tether-store` | SQLite schema, migrations, queries, blob store | `tether-types` |
| `tether-connectors` | `Connector` trait; GitHub implementation; later Telegram, Discord, Google | `tether-types` |
| `tether-mcp` | MCP client: Streamable HTTP transport, stdio transport through Linux mode | `tether-types` |
| `tether-secrets` | Secret references; asks the platform to wrap and unwrap through a callback | `tether-types` |
| `tether-skills` | `SKILL.md` parsing, discovery, on-demand loading, agent-authored skills, install, prune-only upkeep (section 14) | `tether-types` |
| `tether-memory` | Capped agent and user notes, session search, the Obsidian-compatible vault and its link map (section 14) | `tether-types`, `tether-store` |

Rule: only `tether-engine` coordinates other crates. Leaf crates never call each other except where the table lists the dependency.

Status: `tether-types`, `tether-engine`, `tether-ffi`, `tether-store`, `tether-secrets`, and `tether-providers` contain code. The other crates are empty libraries that hold their place in the workspace (see project-structure.md, Present now). P1-11 adds a plain text session actor. P1-15 adds store queries for sessions by project, title, and saved non-rewound text, plus rename. The UI and UniFFI entry points still follow. Each actor owns one store connection and serializes its turns on a Tokio channel. It saves the user message before the request and saves each reply batch before emitting it. A process restart marks an active session `Interrupted`; the transcript, including any saved partial reply, remains readable. `SessionRuntime` exposes create, load, transcript, open, send, and interrupt through UniFFI. The transcript and composer screens follow in P1-12 and P1-13.

`tether-tools` (P2-04, P2-05): `ReadTools` uses the workspace path guard for UTF-8 file reads, direct directory lists, and indexed glob and regex grep. File reads stop at 1 MiB; results stop at 1,000 entries. The engine registers these read tools. `EditTools` uses the same guard for write, exact edit, multi-edit, move, and delete. Multi-edit stages replacements in memory and writes once after every exact match succeeds. Temporary files keep text replacement atomic.

`tether-engine` (P2-06) has a `ToolLoop` that sends file tool schemas, assembles bounded streamed call arguments, validates argument shapes, asks the caller for approval before mutations, dispatches guarded read and edit tools, truncates large results, and repeats provider responses up to 12 exchanges. A scripted fixture drives five calls before a final answer. The session actor persists assistant and tool messages, streams saved text and thinking to the UI, records usage after each completed response, and captures prior file bytes before approved edits. Android opens a tool-enabled actor for a saved project workspace. It shows each tool's target, but denies all mutations until the later approval UI can show and acknowledge the target before work starts. Full JSON Schema validation remains pending.

`tether-secrets` (P1-03): `SecretStore` keeps one file per secret and asks the platform to wrap and unwrap through the `SecretsPlatform` callback, which takes the secret's reference as a second argument and must bind it as authenticated data. The core never holds an encryption key. `Secret` wipes its bytes when dropped and never prints. Writes go to a temporary file with mode 0600 that only that call uses, and are renamed over the target, so a crash leaves the old file or the new one, and two writers to one reference never share a file (the last rename wins). `SecretVault` is the UniFFI face: Kotlin calls `put`, `reveal` (after a biometric check), `has`, and `delete`. Providers will read keys inside the core, not through the vault. The vault is its own object until the engine owns the store; the `init` call in section 8 takes it over then. On Android, `KeystoreSecretsPlatform` in `core-bridge` uses an AES-256-GCM key in the Keystore (StrongBox when available) that needs no user authentication. A process-wide lock guards creating that key, so two platform instances cannot each create one and make the first unreadable.

`tether-providers` (P1-04) holds the provider contract. `ChatProvider::stream_chat(request, events, cancel)` streams one response into a channel as `StreamEvent` values (`TextDelta`, `ThinkingDelta`, `ToolCallStarted`, `ToolCallArgsDelta`, `ToolCallFinished`, `Usage`, `Finished`). `Usage` events carry running totals for the response, with cache read and write tokens kept apart. The last event of a successful stream is always `Finished`. An adapter stops when `cancel` fires or the receiver is dropped, and never retries; the engine retries when the converted `CoreError` is retryable. `ChatRequest` puts the system prompt and tools first so adapters can cache them. `ProviderError` converts to `CoreError`. `SseParser` turns byte chunks into events, follows the WHATWG rules, and drops a line over 1 MiB and an event over 8 MiB.

The Anthropic adapter (P1-05, `tether-providers/src/anthropic/`) posts to `/v1/messages` with `stream: true`. It marks the system block and the last tool with `cache_control`, maps `message_start`, `content_block_*`, `message_delta`, `message_stop`, and `error` events to `StreamEvent` values (unknown event types are ignored), and maps HTTP statuses to `ProviderError` (401 and 403 to `Auth`, 429 to `RateLimited` with `retry-after`, 400, 404, 413, and 422 to `InvalidRequest`, anything else to `Server`). Rewound messages, system notes, and thinking parts are not sent. A request that answers tool calls is sent without `thinking`, because Anthropic wants the signed thinking blocks of the last assistant turn back and Tether does not store the signatures yet. The key is sent in a sensitive header and never appears in `Debug` output or an error. Every provider sends through the client from `http_client()`: HTTPS only, no redirects, a 10 s connect timeout, and a 120 s read timeout.

The adapters share `tether-providers/src/transport.rs`: it sends the request, maps HTTP statuses to `ProviderError`, reads the event stream through `SseParser`, and stops at once when the caller cancels or drops the receiver. An adapter supplies a `StreamMapper` (its events to `StreamEvent` values) and an `ErrorStyle` (how it words errors, and any status it reuses for several failures).

The model catalog (P1-09) uses a bounded JSON GET through the same transport. `ModelCatalog::list_models` returns context limits and tool, image, and reasoning support for the five providers. It keeps provider order and fills missing details and prices from bundled tables; OpenRouter returns its own prices. The store replaces a provider's cached list in one transaction, including an empty list, and records the fetch time separately. A cached list is fresh for 24 hours. Cost math uses cache read and write prices when present, and the input price otherwise.

The OpenAI adapter (P1-06, `tether-providers/src/openai/`) posts to `/v1/responses` with `stream: true` and `store: false`, so OpenAI does not keep the response for later retrieval. That flag does not turn off OpenAI's own abuse-monitoring retention, which can last up to 30 days on a default account; only account-level Zero Data Retention does, and the adapter cannot set it. With a reasoning effort it also asks for a reasoning summary (`summary: "auto"`), because OpenAI sends summary deltas only on request; `OpenAiConfig::reasoning_summary` turns that off for an organization that is not cleared to receive summaries. The system prompt goes in `instructions`, messages become a flat list of input items, and a tool call is a `function_call` item without an `id`, because an `id` makes OpenAI demand the reasoning item Tether does not store. OpenAI caches the stable prefix on its own, so there are no cache markers. It maps `response.output_text.delta`, reasoning summary deltas, function call argument deltas, `response.completed`, `response.incomplete`, `response.failed`, and `error` events, and ignores unknown types. Cached input tokens are split out of the input count. A 429 with the code `insufficient_quota` is `Auth`, because a billing problem is not retryable.

The Gemini adapter (P1-07, `tether-providers/src/gemini/`) posts to `/v1beta/models/<model>:streamGenerateContent?alt=sse`. The key goes only in the `x-goog-api-key` header, never in the URL, and a model id with any character other than letters, digits, `.`, `-`, and `_` is rejected so it cannot change the path. Tool schemas go in `parametersJsonSchema`, which takes full JSON Schema (`additionalProperties` and the like); the older `parameters` field accepts only Google's restricted schema and would fail with a 400. Messages become `contents` with the roles `user` and `model`, and a tool result finds its function name from the earlier call with the same id. Each stream chunk is a full response: function calls arrive whole and become a started, arguments, and finished event, and a call that has no `id` gets one built from the current time, a process-wide counter, and its position in the response (`gemini_<ms>_<n>_<i>`), so ids stay unique across responses and restarts; parts marked `thought` become thinking deltas; usage counts thinking tokens as output and splits out cached tokens. There is no end event, so the stream is finished by a `finishReason` (the placeholder `FINISH_REASON_UNSPECIFIED` does not count) or a blocked prompt. HTTP 400 with "API key not valid" is `Auth`. Google does not document a way to skip thought signatures, and Tether does not store them, so a tool loop on a model that requires them may fail until reasoning replay is built (see the tracker notes).

The OpenAI-compatible adapter (P1-08, `tether-providers/src/openai_compat/`) speaks Chat Completions and serves DeepSeek, OpenRouter, and any other compatible server through three settings: `OpenAiCompatConfig::deepseek`, `::openrouter`, and `::custom`, each with a `Dialect`. It posts to `{base_url}/chat/completions` with `Authorization: Bearer`, `stream: true`, and `stream_options.include_usage`. A message with several tool results becomes one `tool` message per result. An extra header cannot replace `Authorization`. The dialect decides the reasoning field (`reasoning_effort` for plain servers, `reasoning: {effort}` for OpenRouter, none for DeepSeek), which cache field counts (`prompt_cache_hit_tokens` for DeepSeek, `prompt_tokens_details.cached_tokens` otherwise), and whether OpenRouter is asked for usage. The stream ends with `[DONE]`, and a body that ends before it is malformed. Tool calls assemble by `index`; a call without an id gets one built from a per-response prefix. `reasoning_content` (DeepSeek) and `reasoning` (OpenRouter) become thinking deltas. HTTP 402 (no balance or credits) is `Auth`, because it is not retryable. OpenRouter sends mid-stream errors as an `error` object in a chunk. For DeepSeek, an assistant message in history that has thinking text is sent back with `reasoning_content` whenever the request carries tools, because DeepSeek answers HTTP 400 without it (the text is plain, so unlike Anthropic and Gemini signatures it can be replayed). OpenRouter cache writes (`prompt_tokens_details.cache_write_tokens`) are counted apart from ordinary input, and `delta.refusal` is forwarded as text. A `null` where the adapter expects a count, an object, or a list is read as empty, because some servers send `null` for fields that are normally absent.

### Android (`android/`)

| Module | Owns |
|---|---|
| `app` | Activity, navigation graph, dependency setup, flavors, and the placeholder screens in `shell/` until feature modules exist |
| `core-bridge` | Generated UniFFI Kotlin, the `CoreClient` wrapper that turns callbacks into `Flow`s, platform callback implementations |
| `designsystem` | Theme, tokens (generated from `tokens/tokens.json`), bundled fonts, shared components |
| `feature-onboarding` | First-run screens |
| `feature-projects` | Project list, create, clone, import, project home (P2-13 adds a detail screen over the current home sources; persisted project data remains) |
| `feature-session` | Transcript, composer, approvals, status line, pickers |
| `feature-diff` | Diff view, hunk actions, checkpoints list |
| `feature-files` | File tree, file viewer |
| `feature-runs` | Run console, Linux mode setup, CI runs list |
| `feature-settings` | All settings screens |
| `service` | `AgentService`, notification channels, notification actions |
| `platform` | Keystore wrapping, biometric prompt, speech input, file picker, share intents |

Feature modules depend on `core-bridge` and `designsystem`, never on each other. Shared navigation routes live in `app`.

Status: at P0 the app shell is an exception to the rule that composables never call the core and the core is never called on the main thread: `TetherApp` starts the core and the Projects screen reads the version and sends the test event directly. All three calls return at once. The first feature screens (P1-11) move core calls behind ViewModels and `Dispatchers.IO`. At P0 the Gradle build has three modules: `app`, `core-bridge`, and `designsystem`. The `feature-*`, `service`, and `platform` modules are added in the phase that first needs each one. Convention plugins live in `android/build-logic` as precompiled script plugins.

## 4. Internal communication

- **Kotlin to Rust:** UniFFI async functions on a `TetherCore` object. Kotlin calls them from coroutines; UniFFI bridges Rust futures to suspend functions.
- **Rust to Kotlin:** one `EventSink` callback interface. The core sends `CoreEvent` values; `CoreClient` forwards them into a `MutableSharedFlow` with a 256-event buffer and `DROP_OLDEST` only for `TextDelta` events (other events never drop). ViewModels filter by session ID.
- **Rust to platform services:** callback interfaces implemented in Kotlin: `SecretsPlatform` (wrap, unwrap with Keystore), `NotifyPlatform` (approval requests when backgrounded), `NetworkPlatform` (connectivity changes).
- **Streaming text:** the core batches text deltas every 32 ms or 64 characters, whichever comes first, so Compose recomposes at most about 30 times a second.
- **Inside the core:** tokio channels. Each session has one actor task that owns its state; commands arrive on an `mpsc` channel, which serializes all changes to a session.

## 5. Key flows

### 5.1 One turn

1. UI calls `send_message(session_id, text, attachments)`.
2. Session actor stores the message, builds the request: system prompt, instructions, tool schemas, context, message.
3. Provider adapter streams the response. Text deltas become `TextDelta` events.
4. For each tool call in the response: validate arguments against the schema; run the path guard; check permissions.
   - Allowed: run the tool, emit `ToolStarted` and `ToolFinished`.
   - Needs approval: emit `ApprovalRequired`; the actor waits for `approve` or `deny`.
   - Blocked: return an error result to the model explaining the mode.
5. If the tool calls included edits, close the edit batch: write the checkpoint, run the quality gate, emit `DiffReady` and `QualityReport`.
6. Send tool results back to the model; repeat from step 3 until the model ends the turn or the user interrupts.
7. Store usage; emit `UsageUpdated`, then `TurnFinished`.

### 5.2 Approval while backgrounded

1. Actor emits `ApprovalRequired`.
2. `CoreClient` sees no foreground activity; calls `NotifyPlatform.show_approval`.
3. Notification shows the action summary with Allow and Deny.
4. Tapping an action sends a `PendingIntent` to `ApprovalReceiver`, which calls `approve` or `deny` on the core.
5. Actor resumes the turn and updates the service notification.

### 5.3 Executor routing

1. `run` arrives with a command and optional executor.
2. If an executor is named and available, use it.
3. Else read `.tether/config.toml` `[exec]` rules: first matching pattern wins.
4. Else use built-in rules: sandbox for commands whose program is in an installed runtime pack; Linux mode for package managers and compilers if installed; CI otherwise.
5. If the chosen executor is unavailable (for example Linux mode not installed in the `play` flavor), fall to the next and include the reason in the tool result.

### 5.4 CI run

1. Ensure `.github/workflows/tether.yml` exists on the work branch; if not, propose adding it as an edit.
2. Commit pending changes to the work branch (with approval per mode) and push.
3. Dispatch the workflow with the command as input; read the run ID.
4. Poll every 10 s with backoff up to 60 s; emit `RunProgress` per job step.
5. On completion, fetch logs, trim to the failing step, parse file and line references, return the result to the model.

### 5.5 Rewind

1. User picks a checkpoint and a restore option.
2. Actor stops any running turn.
3. Files: write back each blob; delete files that did not exist at the checkpoint.
4. Conversation: mark later messages as rewound (kept in the database, hidden from context).
5. Emit `SessionRewound`; UI reloads the transcript.

P2-08 lists checkpoints newest first. The session actor cancels an active turn before it handles a rewind command. File restore reads every checkpoint from the selected batch onward in reverse edit order, verifies all blobs and guarded paths before writing, restores prior bytes through temporary files, and removes files that did not exist before an edit. Conversation restore marks the checkpoint's assistant call and all later messages as rewound. New provider requests omit those messages. The UniFFI session runtime lists checkpoints, and its session runner accepts all three restore options. A conversation-only rewind works without a workspace runtime.

### 5.6 Linux mode setup

1. User enables Linux mode in Settings (`full` flavor only).
2. Core fetches the signed manifest, verifies the signature with the public key compiled into the app.
3. Downloads the Alpine mini root filesystem, checks its SHA-256 against the manifest, extracts it under `linux/rootfs/alpine/`.
4. Runs `apk add` for the base package list through PRoot, with network allowed for setup only.
5. Marks the environment ready and stores the Alpine version.

## 6. State machines

### Session

```
Idle ──send──▶ Requesting ──first token──▶ Streaming
Streaming ──tool call──▶ CheckingTool
CheckingTool ──allowed──▶ RunningTool ──done──▶ Requesting
CheckingTool ──needs approval──▶ AwaitingApproval ──approve──▶ RunningTool
AwaitingApproval ──deny──▶ Requesting (with denial result)
Streaming ──end of turn──▶ Idle
any active state ──interrupt──▶ Idle (turn marked Interrupted)
any active state ──network lost──▶ Paused ──network back──▶ previous state
any active state ──process death──▶ (on relaunch) Interrupted ──resume──▶ Requesting
any active state ──fatal error──▶ Failed ──retry──▶ Requesting
```

### Tool call

`Proposed → (Allowed | AwaitingApproval) → (Running | Denied | Blocked) → (Succeeded | Failed | TimedOut | Cancelled)`

### CI run

`Preparing → Pushing → Dispatched → Queued → InProgress → (Succeeded | Failed | Cancelled | TimedOut)`

### Linux environment

`NotInstalled → Downloading → Verifying → Extracting → InstallingPackages → Ready`; any step can go to `Broken`, from which **Repair** restarts at `Downloading` and **Remove** returns to `NotInstalled`.

### Provider connection

`Unset → (Ok | BadKey | NetworkError | Blocked)`. `Testing` is a screen state only: it is never written to the database, so a test that dies with the app cannot leave a provider stuck on "testing" (a stored `testing` reads back as `Unset`). Saving a new key, or removing the key, returns the provider to `Unset`.

P1-10 tests a saved key with the provider's model-list request (`ModelCatalog::list_models`), which returns data without generating tokens. OpenRouter's model list is public and answers 200 for any key, so its test first calls `GET /key` (`ModelCatalog::verify_key`), which needs a valid key, and lists the models only when that succeeds; the screen shows both URLs. The check lives in `tether-engine` (`check_provider`) and runs for at most 20 seconds. A blank key is `BadKey` with no request. The result maps as follows:

| Result | Status |
|---|---|
| Model list returned | `Ok`, and the list is cached (24 hours, see the `models` table) |
| HTTP 401, or 403 | `BadKey` |
| OpenAI 403 with code `unsupported_country_region_territory` or a "country, region, or territory" message; Gemini 400 or 403 with a "location is not supported" message | `Blocked` |
| Network failure, timeout, server error, rate limit, bad response | `NetworkError` |

Only those two documented signals count as a region block, because for Anthropic, OpenRouter, DeepSeek, and Gemini a 403 means a permission or key problem. `ProviderSettings` (`tether-ffi`) reads the wrapped key through `SecretVault`, calls the engine, and saves the status, the test time, and the key reference in `provider_configs`; the key itself never leaves the core except through `reveal_key`, which Kotlin calls only after a biometric or screen lock check. `test_target` returns the exact URL of the request, so the screen can show it before the user taps Test connection. Removing a key also clears that provider's cached models.

### App update

`Idle → Checking → (UpToDate | Available) → Downloading → Ready → (NeedsPermission →) Installing → Installed`; any step can go to `Failed`, from which the screen offers the step that failed again. On the Play build `Available` ends at an "Open Google Play" button: that build never downloads or installs.

The check, download, and verification live in `tether-engine` (`updates`) and run for the Kotlin layer through the `Updater` object in `tether-ffi`:

- **Check.** `GET https://api.github.com/repos/khanblair/tether/releases/latest` with no credentials and no user data (the only identifying header is `User-Agent: tether-updater`). Drafts and pre-releases are ignored (releases are never pre-releases, D21). The tag must be `v` plus three numbers; versions compare as numbers, so `0.0.10` is newer than `0.0.9`. A newer release needs an asset named exactly `tether-full-v<version>.apk` and one named exactly `SHA256SUMS`. The check is limited to 20 seconds and the response to 1 MiB.
- **Download and verify.** The core fetches `SHA256SUMS` (up to 64 KiB) and takes the one line for the APK name, streams the APK into a `.part` file while hashing it, and keeps it only when the SHA-256 matches. Wrong digest, wrong size, a stream longer than the published size, a file over 256 MiB, or a cancel deletes the partial file. File names are built only from the strict `tether-full-v<digits>.<digits>.<digits>.apk` pattern. A file that was already downloaded and still matches is reused. Older downloads are removed.
- **Network rules.** HTTPS only. Redirects (GitHub serves the asset through a storage host) are followed at most 5 times and only to `github.com`, `api.github.com`, or a host ending in `.github.com` or `.githubusercontent.com`; no `Authorization` header is ever sent. The app never sends URLs back to the core: `download(version)` uses the release the last `check` found.
- **Install.** Kotlin hands the verified file to `PackageInstaller` (`full` build only, which declares `REQUEST_INSTALL_PACKAGES`; the Play build does not). Android shows its own confirmation and refuses an APK signed with a different key, so a swapped file cannot replace the app. The first install asks the user to allow installs from Tether in system settings.
- **When it runs.** `UpdatesController` checks in the background when the app starts if the last check is older than 12 hours (failures stay silent), and again whenever About opens. A newer version shows as a dot on the bell and an Updates item in Needs attention, and in Settings.

## 7. Data model

SQLite database at `files/db/tether.db`, WAL mode, foreign keys on. IDs are ULIDs stored as text. Times are Unix milliseconds.

| Table | Columns | Notes |
|---|---|---|
| `projects` | id, name, path, remote_url, default_branch, default_provider, default_model_id, default_mode, created_at, opened_at | `path` is relative to `files/workspaces/`; a `CHECK` rejects empty, absolute, and `..` paths |
| `sessions` | id, project_id, title, provider, model_id, mode, status, created_at, updated_at, cost_micros, tokens_in, tokens_out | `cost_micros` is USD × 1,000,000 |
| `messages` | id, session_id, role, seq, created_at, rewound | role: user, assistant, tool, system_note |
| `message_parts` | id, message_id, seq, kind, body_json | kind: text, image_ref, tool_call, tool_result, thinking, compaction_summary |
| `tool_calls` | id, message_id, tool, args_json, status, executor, started_at, finished_at, result_ref | `result_ref` points to a blob for large results |
| `approvals` | id, tool_call_id, decision, reason, decided_at, via | via: app, notification |
| `permission_rules` | id, project_id, tool, pattern, executor, created_at | from "Always allow this" |
| `checkpoints` | id, session_id, message_id, created_at | one per edit batch |
| `checkpoint_files` | checkpoint_id, path, blob_hash, existed | `existed = 0` means the file was created in the batch |
| `quality_reports` | id, checkpoint_id, findings_json, blocking_count | |
| `runs` | id, tool_call_id, executor, command, exit_code, duration_ms, output_ref, ci_run_id | |
| `provider_configs` | provider, secret_ref, base_url, status, tested_at | no key material; `secret_ref` names the wrapped secret |
| `models` | provider, model_id, display_name, context_window, max_output_tokens, input_price_micros, output_price_micros, cache_read_price_micros, cache_write_price_micros, price_source, caps_json, position | catalog rows in provider order; prices are USD × 1,000,000 per million tokens, NULL when unknown |
| `models_fetch` | provider, fetched_at | last successful catalog fetch, including an empty list; Unix milliseconds |
| `mcp_servers` | id, name, url, transport, secret_ref, enabled | |
| `usage_daily` | day, provider, model_id, project_id, tokens_in, tokens_out, cost_micros | for charts and budgets; `day` is a UTC date, `YYYY-MM-DD`; no foreign key on `project_id`, so cost history outlives a deleted project |
| `message_fts` | FTS5 virtual table over text parts: part_id, session_id, message_id, body | migration 0002 backfills rows and triggers track edits and rewinds; bundled SQLite with FTS5 |
| `vault_notes` | path, title, description, type, tags_json, updated_at, hash | rebuildable index of the vault folder |
| `vault_links` | src_path, dst_path, heading, kind, resolved | wikilinks and backlinks; `resolved = 0` means the target note does not exist yet |
| `skill_index` | name, scope, path, description, last_used_at, state | state: active, stale, archived; rebuildable from the skill folders |

P2-07 stores edit snapshots as zstd-compressed files under a private blob folder, named by BLAKE3 hash. The engine captures prior bytes through `PathGuard` before approved tool edits; the store writes blobs and `checkpoints` and `checkpoint_files` rows, including paths that did not exist. Identical content has one file. Cleanup of unreferenced blobs and shared storage for large tool results and run output remain pending.

Migrations are numbered SQL files in `core/crates/tether-store/migrations/`. At core start each one newer than the database runs in its own transaction, and the version is kept in `PRAGMA user_version`, written in the same transaction, so a failed migration leaves the database at the version before it. Foreign keys are off while a migration runs, because rebuilding a parent table to change a `CHECK` list would otherwise delete its children, and `PRAGMA foreign_key_check` must pass before the migration commits. A second opener that waited for the write lock skips migrations already applied. A database newer than the app is refused. Tables use `STRICT`, and enum columns have `CHECK` lists that a store test keeps equal to the text forms in `tether-types`.

Status: schema 0001 (P1-02 and P1-09) creates `projects`, `sessions`, `messages`, `message_parts`, `provider_configs`, `models`, `models_fetch`, and `usage_daily`. The other tables arrive in numbered migrations with the task that first uses them. Until a release opens the database, 0001 may change; after that, only new migrations may.

## 8. API

The core's API is the UniFFI surface. There are no HTTP routes.

### Commands (Kotlin calls Rust)

| Function | Purpose |
|---|---|
| `init(config: CoreConfig, sink: EventSink, platform: PlatformCallbacks)` | Start the core |
| `list_projects()` / `create_project(spec)` / `clone_project(url, opts)` / `import_zip(fd)` / `delete_project(id)` | Projects |
| `list_sessions(project_id)` / `create_session(project_id, opts)` / `load_session(id)` | Sessions |
| `send_message(session_id, input: UserInput)` | Start a turn |
| `approve(tool_call_id, scope: ApprovalScope)` / `deny(tool_call_id, reason)` | Approvals |
| `interrupt(session_id)` / `resume(session_id)` | Turn control |
| `set_mode(session_id, mode)` / `set_model(session_id, model)` | Session settings |
| `list_checkpoints(session_id)` / `rewind(checkpoint_id, what)` | Rewind |
| `get_diff(project_id, scope)` / `apply_hunk_action(...)` | Diff review |
| `run_command(project_id, cmd, executor)` | User-initiated run |
| `set_provider_key(provider, key)` / `test_provider(provider)` / `list_models(provider)` | Providers |
| `github_device_start()` / `github_device_poll(code)` / `github_set_token(token)` | GitHub auth |
| `linux_install()` / `linux_repair()` / `linux_remove()` / `linux_status()` | Linux mode |
| `runtime_packs()` / `install_runtime_pack(id)` | Sandbox runtimes |
| `add_mcp_server(spec)` / `remove_mcp_server(id)` | MCP |
| `usage(range, group_by)` | Cost data |
| `export_session(id)` | Markdown export |

### Events (Rust calls Kotlin through `EventSink.on_event`)

Every event for one session carries its `session_id`, so ViewModels can filter on it. `SessionUpdated` is the source of truth for session state; the turn and network events add detail the state cannot hold, and the engine emits them from the same transition so they never disagree.

`SessionUpdated`, `TextDelta`, `ThinkingDelta`, `ToolProposed`, `ApprovalRequired`, `ToolStarted`, `ToolOutput` (streaming run output), `ToolFinished`, `DiffReady`, `QualityReport`, `PlanReady`, `TodoUpdated`, `AskUser`, `RunProgress`, `CompactionDone`, `UsageUpdated`, `TurnFinished`, `TurnInterrupted`, `TurnFailed`, `SessionRewound`, `LinuxStatus`, `RuntimeStatus`, `ProviderStatus`, `NetworkPaused`, `NetworkResumed`.

Status: P1-01 defines `Ready` and `Test` (from P0), `SessionUpdated`, `TextDelta`, `ThinkingDelta`, `ToolProposed`, `ApprovalRequired`, `ToolStarted`, `ToolOutput`, `ToolFinished`, `UsageUpdated`, `TurnFinished`, `TurnInterrupted`, `TurnFailed`, `NetworkPaused`, `NetworkResumed`, and `ProviderStatus`. `DiffReady`, `QualityReport`, `PlanReady`, `TodoUpdated`, `AskUser`, `RunProgress`, `CompactionDone`, `SessionRewound`, `LinuxStatus`, and `RuntimeStatus` arrive with the task that first emits them.

### Shared types

Defined in `tether-types` as plain Rust with no UniFFI code. `tether-ffi` converts the types the UI sees into UniFFI view types with a `View` suffix (`CoreEventView`, `MessageView`, `ModelRefView`, and so on). Ids cross the boundary as plain strings, and the Kotlin layer wraps them in value classes where it keeps them.

Present now: `ProjectId`, `SessionId`, `MessageId`, `ToolCallId` (newtypes over strings), `Role`, `MessagePart`, `Message`, `PermissionMode`, `Executor`, `Provider`, `ModelRef`, `SessionStatus`, `ToolCallStatus`, `ConnectionStatus`, `UsageSnapshot`, and `CoreError`. Later tasks add `UserInput`, `Attachment`, `ToolCallView`, `DiffSummary`, `Hunk`, `Finding`, and `Severity`.

`CoreError` is an enum with one variant per kind: `Internal`, `Network`, `Auth`, `RateLimited`, `InvalidInput`, `NotFound`, and `Cancelled`. Each variant carries a `message`. `retryable()` is true for `Network` and `RateLimited`. Over the FFI it is `CoreErrorView`, a record with `kind`, `message`, and `retryable`. Events such as `TurnFailed` carry it. It is a record and not a Kotlin exception, so events compare by value. The error type that commands return arrives with the first command that fails.

Rule: every type the UI sees is a view type built for display. Internal engine types do not cross the FFI boundary.

## 9. Files on disk

All under the app's private `files/` directory unless noted.

| Path | Contents |
|---|---|
| `workspaces/<project-id>/` | Project files and `.git` |
| `db/tether.db` | SQLite database |
| `blobs/<aa>/<hash>` | Content-addressed blobs |
| `secrets/<ref>.bin` | The bytes `SecretsPlatform.wrap` returned (AES-GCM with a Keystore key: 12-byte IV, ciphertext, tag), after a 5-byte header (`TSEC` and a format version). `<ref>` allows only `a-z`, `0-9`, `-`, and `_`, up to 64 characters |
| `memory/MEMORY.md`, `memory/USER.md` | Agent notes and user profile, capped in size (F22) |
| `memory/vault/` | The Obsidian-compatible vault: plain Markdown notes, `index.md`, `log.md` (F23) |
| `skills/<category>/<name>/SKILL.md` | User skills, with optional `references/`, `templates/`, `scripts/`, `assets/` (F21) |
| `skills/.quarantine/`, `skills/.archive/` | Skills awaiting approval; skills archived by the upkeep job |
| `runtimes/<pack>/<version>/` | Runtime pack `.wasm` files and the precompiled `.cwasm` for this device |
| `linux/rootfs/alpine/` | Alpine root filesystem (`full` flavor) |
| `linux/tmp/` | PRoot temp directory |
| `logs/core-<date>.log` | Rotating core logs, 7 days, 5 MB each, secrets redacted |
| `cache/` (Android cache dir) | Model catalog, fetched web pages; Android may clear it |

Backups: `secrets/`, `linux/`, `runtimes/`, `blobs/`, `logs/`, and `cache/` are excluded from Android backup and device transfer. Workspaces are excluded by default with a setting to include them. `memory/` and `skills/` are included, because they are the user's own notes; they never hold secrets (writes are scanned, section 14).

## 10. Security model

**Assets:** API keys and GitHub token; the user's code; the user's money (provider usage, Actions minutes); remote repos.

**Threats and controls:**

| Threat | Control |
|---|---|
| Key theft from storage | Keys encrypted with an AES-GCM key held in Android Keystore (StrongBox when available); never written in plaintext; excluded from backup |
| A wrapped secret file copied over another reference (for example the GitHub token over a provider key, to send it to a third party) | The reference is passed to the platform as authenticated data when wrapping and unwrapping, so a file moved to another reference fails to unwrap |
| Keys leaking into logs, transcripts, exports | `tether-secrets::Redactor` masks known key formats and registered values. Logger and exporter integration remains in P1-18; S7 is not met yet |
| Agent-run code reading keys | Secrets never enter executor environments; sandbox sees only `/workspace`; Linux mode binds only the project folder and a temp folder |
| Path escape | Path guard canonicalizes every path, rejects `..` outside the root, rejects symlinks pointing outside, rejects writes inside `.git/` |
| Prompt injection via repo files or web content | Network tools, pushes, and non-sandbox runs need approval below Bypass; tool results are marked as data in the prompt; no tool can change permission settings |
| Destructive remote actions | Push to default branch, force push, and remote branch delete always ask (force push asks even in Bypass) |
| Tampered runtime packs or root filesystem | Signed manifest (Ed25519, key in the app) lists SHA-256 for every download; mismatch aborts |
| Tampered update APK | The Rust core downloads the APK and checks its SHA-256 against the release `SHA256SUMS`; the Kotlin layer then hands it to `PackageInstaller`, and Android rejects an APK signed with a different key |
| Runaway cost | Budgets with pause; cost visible at all times |
| Shoulder surfing and screenshots | Optional `FLAG_SECURE`; key reveal needs biometric |
| Malicious MCP server | Servers added only by the user; each tool approval-gated by default; server output treated as data |

Network: all HTTPS through rustls with platform roots. No cleartext traffic allowed in the Android network security config.

## 11. Performance budgets

| Area | Budget |
|---|---|
| Cold start to first frame | Under 800 ms on the reference device; core init is lazy and off the main thread |
| Main thread | No FFI call on the main thread; all core calls from `Dispatchers.IO` or a dedicated dispatcher |
| Event delivery | Core to UI under 16 ms at p95 |
| Text streaming | Max 30 UI updates per second per session |
| Transcript | `LazyColumn` with stable keys; message models immutable; code blocks highlighted once in the core and cached |
| Sandbox | Engine and precompiled modules cached; instance creation under 100 ms p95 |
| Search | Index in memory per open project; grep runs on the blocking pool with a 2 s cap and partial results |
| Memory | Under 300 MB app total excluding Linux mode; tool results over 256 KB stored as blobs, not held in memory |
| Database | Writes batched per event; no query over 20 ms on the main session paths |
| APK size | Base APK under 35 MB per ABI; runtime packs downloaded on demand |

## 12. Key decisions and trade-offs

| Decision | Chosen | Alternatives | Trade-off accepted |
|---|---|---|---|
| Core language | Rust | Kotlin only; Go | More build complexity for a core that ports to iOS unchanged and runs Wasmtime and gitoxide natively |
| UI | Native Compose, later SwiftUI | Compose Multiplatform; Flutter; React Native | Two UI codebases later, for the best platform behavior (services, notifications, text input) |
| Bindings | UniFFI | Hand-written JNI | Some type-shape limits, for generated Kotlin and Swift from one definition |
| Default executor | WASM sandbox | Linux mode by default | Narrower runtime support, for speed, isolation, and iOS portability |
| Linux mode | PRoot + Alpine, opt-in, `full` flavor | No Linux mode; full VM | Platform and policy risk, for real tooling without a PC |
| Heavy builds | GitHub Actions | Own cloud runners | Needs a GitHub repo and minutes, for zero infrastructure |
| Git | gitoxide | libgit2 via `git2` | Some features still maturing in gitoxide, for a pure Rust build with no C dependency |
| Storage | SQLite + blob files | An object database; flat JSON | Schema migrations to maintain, for queries and reliability |
| Server | None | Relay server for chat connectors and sync | Chat connectors only work while the phone runs, for no hosting cost and no data custody |
| Provider calls | From Rust | From Kotlin with OkHttp | Streaming code lives in Rust, so iOS reuses it |

## 13. Automated quality checks

Two separate systems: the in-app quality gate that checks the user's code, and the CI checks on Tether's own repo.

### In-app quality gate (product feature F12)

- Runs in `tether-quality` after each edit batch, on changed line ranges plus enclosing functions.
- Tree-sitter grammars bundled for: Python, JavaScript, TypeScript, TSX, Rust, Go, Kotlin, Java, Swift, C, C++, Ruby, PHP, HTML, CSS, JSON, YAML, TOML, Markdown, Bash.
- Rules are Rust structs implementing a `Rule` trait: `id`, `languages`, `default_severity`, `check(tree, source, changed_ranges) -> Vec<Finding>`.
- Slop word lists are data files (`core/crates/tether-quality/data/slop_words.txt`, `slop_patterns.toml`) so they can update without code changes.
- Budget: under 300 ms for a 20-file batch on the reference device; rules that exceed 50 ms on a file are skipped for that file and noted in the report.
- Thresholds and severities are overridable in `.tether/config.toml` under `[quality]`.

### Tether repo CI (`.github/workflows/ci.yml`)

| Job | Runs |
|---|---|
| `rust` | `cargo fmt --check`, `cargo clippy -D warnings` (which carries the smell thresholds, proven by a fixture crate that must fail it), `cargo test`, `cargo ndk check` for both Android ABIs (bundled SQLite compiles C code), `cargo deny check` |
| `tooling` | Token generation is current and every contrast pair passes; `tools/slop-lint` tests, then slop-lint over `docs/`, `README.md`, and `strings.xml`; the file length check and its self test; the commit size check self test, and on pull requests no commit touching more than 15 files; the docs site builds and every link between its pages resolves |
| `bindings` | Regenerate UniFFI bindings and fail if they differ from committed output |
| `android` | `ktlint` and `detekt` (through `qualityCheck`; a fixture module must fail detekt), Android lint with warnings as errors, unit tests, `assemble` both flavors; uploads the debug APKs |

Not in CI yet: instrumented and Compose UI tests on an emulator (run locally with `just test-device`), the `bench` job, and the nightly live provider smoke tests. Slop-lint does not yet check code comments or commit messages.

Other workflows: `pr.yml` runs the `issue-link` check on every pull request and on every edit of its description. `pages.yml` builds and deploys the docs site on changes to docs, images, tokens, or the site tools. `release.yml` runs on every push to `main`. When the version in `core/Cargo.toml` has no tag yet, it publishes a signed GitHub Release (see development.md section 13).

`tools/slop-lint` reuses the same rule data as the in-app gate, so the product and the repo follow one definition of AI slop.

## 14. Skills, memory, and the vault

Three features that let the agent keep what it learns. They borrow designs from Hermes Agent (MIT) and from the Obsidian vault format, and they stay inside the phone-only rule: all of it is plain files plus SQLite on the device.

### Skills (F21, `tether-skills`)

A skill is a folder with a `SKILL.md`: YAML frontmatter, then Markdown instructions. P11-01 starts with a scalar frontmatter parser and validator; block scalars and full YAML syntax remain to be added. The format follows the open Agent Skills standard (agentskills.io), so skills written for other agents load unchanged.

| Frontmatter field | Rule |
|---|---|
| `name` | Required. Up to 64 characters, lowercase letters, digits, and hyphens. Must equal the folder name |
| `description` | Required. Up to 1,024 characters. Says what the skill does and when to use it |
| `license`, `compatibility`, `allowed-tools` | Read and shown; not enforced |
| `metadata` | String keys to string values. Tether-only settings use a `tether-` key prefix, so a skill stays valid elsewhere |

Locations, in order of precedence: project skills in `<workspace>/.tether/skills/` and `<workspace>/.agents/skills/` (a project skill needs the user's trust approval before it loads), then user skills in `files/skills/`, then bundled skills.

**Progressive disclosure keeps the prompt small.**
1. The system prompt carries only an index: name, description, and category per skill, under a fixed token budget.
2. `skill_view(name)` loads the full `SKILL.md`.
3. `skill_view(name, path)` loads one reference file.

A skill added mid-session is announced in the next user message instead of rewriting the system prompt, so the prompt cache stays valid.

**The agent writes skills.** `skill_manage` supports `create`, `patch` (targeted find and replace, preferred), `rewrite`, `delete`, `write_file`, and `remove_file`. The system prompt tells the agent to save a skill after a multi-step workflow worth repeating, after an error it had to work around, and after the user corrects it. Rule for content: lessons, not logs. An advisory check warns when a skill body passes 24,000 characters or it has more than 60 reference files.

**Writes need approval by default.** Unlike Hermes, Tether stages agent-authored skill writes and shows them as a diff in the approval flow (principle: show before doing). A setting turns this off.

**Install.** A skill can be installed from a GitHub URL or a zip into `skills/.quarantine/`. Tether shows its files, runs a static scan (instructions to read secrets or send data elsewhere, shell downloads, hidden Unicode), and installs only on approval. Skill scripts never run outside the executors, and the router and permission modes apply to them like any command.

**Upkeep.** A prune-only job marks a skill stale after 14 days unused and moves it to `skills/.archive/` after 30. It never deletes, never touches pinned or hand-written skills, and makes no model calls.

### Memory (F22, `tether-memory`, `tether-store`)

Two small files plus search.

- **`MEMORY.md`** holds the agent's notes: environment facts, conventions, lessons. Capped at 2,200 characters. **`USER.md`** holds the user profile and preferences. Capped at 1,375 characters. Entries are separated by a section sign.
- **The `memory` tool** has `add`, `patch`, and `remove`, each with a target of `memory` or `user`. P11-08 adds the file store and validation; engine tool dispatch remains to be wired. There is no read action, because the content is already in the prompt. A write over the cap returns an error, and the agent must trim entries in the same turn. Exact duplicates are skipped. Every write is scanned for prompt-injection phrases, credential patterns, and invisible Unicode, and rejected on a hit.
- **Frozen snapshot.** P11-09 adds an immutable snapshot and labeled system block; the session actor does not yet use it. The target flow reads files once at session start and places them in the system prompt. Writes reach disk at once but enter the prompt only at the next session. This keeps the cached prompt prefix stable, which lowers cost on every provider that caches.
- **Session search.** Every message is indexed in SQLite FTS5 (`message_fts`). `session_search` returns the matching messages, not model summaries, and the agent can read around a hit. No model call is involved.

There is no external user-modeling service. Tether keeps no server, and extra model passes would cost the user tokens.

### Vault (F23, `tether-memory/vault`)

Long-term notes that outgrow two capped files go into a vault: a folder of plain Markdown that follows Obsidian's format, so it opens in Obsidian or any editor.

| Part | Rule |
|---|---|
| Notes | Markdown with YAML frontmatter. `description` is required. `type`, `tags`, `aliases`, `created`, and `updated` are used when present |
| Links | `[[Note]]`, `[[Note#Heading]]`, and `[[Note|alias]]`. Backlinks are computed |
| `index.md` | A catalog of notes by category, one line each. The agent reads it first |
| `log.md` | Append-only, dated entries. A correction is a new entry, never an edit |
| Map notes | A note with `type: moc` that only groups links for one area, such as a project. It gives the agent a bounded entry point instead of a vault-wide scan |
| `.obsidian/` | Never written by Tether |

**The map.** P11-11 first adds a file-based, rebuildable note and link view. P11-12 adds core file operations and bounded map output; agent tool dispatch and write scanning remain. The SQLite `vault_notes` and `vault_links` tables are still pending. The target design parses every note into `vault_notes` and `vault_links`. Both tables can be rebuilt from the files at any time, so the folder is the source of truth. The vault tools are `vault_search` (full text, tags, and links), `vault_read`, `vault_write` and `vault_append`, `vault_links` (outlinks and backlinks of a note), `vault_orphans`, and `vault_map`, which returns the neighborhood of a note out to a given depth within a token budget. The app shows the same data as a list view of map notes and neighbors. A force-directed graph view is not planned.

**Start of session.** A small injector adds a slice to the prompt: the head of `index.md`, the map note of the active project, and the last entries of `log.md`, within a fixed budget. The agent fetches full notes on demand.

**Checks on every write.** The note must have a `description`. Links that point nowhere are recorded as unresolved, not rejected. Size is capped. A note that nothing links to is reported by `/vault lint`.

**Where the vault lives.** P11-15 adds a copy-based sync contract and fake-folder tests. Android document-tree access, manifest persistence, and zip import/export remain. The vault is in app-private storage (`files/memory/vault/`), because the Rust core cannot open Android content URIs and so cannot read a shared folder directly. Two ways in and out:
1. Export and import as a zip through the system file picker.
2. Folder sync: the user picks a shared folder (Storage Access Framework). The Kotlin platform layer copies changed files both ways, comparing hashes and modification times, when the user asks and after each turn that wrote to the vault. This lets Obsidian for Android open the same notes. The vault tools only ever see the private copy.

**Trust.** Vault and memory text is data, not instructions. It enters the prompt inside labeled blocks, and the system prompt says to treat it as untrusted. Writes are scanned like memory writes. Secrets never go in: the redaction layer from the S7 test covers memory and vault files.

### Phones are not desktops

Not copied from the sources: Honcho-style user modeling, LLM consolidation passes, a skills hub with remote browsing, the Obsidian graph view, Dataview, the Local REST API plugin, and desktop hooks.

