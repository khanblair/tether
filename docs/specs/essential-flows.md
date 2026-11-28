# Essential flows: scope for first run and core sessions

Status: built on branch `feat/essential-flows` on 26 Nov 2026 (see section 9). Original scope written 24 Nov 2026. Base: `main` at `a89f04a` (v0.0.9). The phone that hit the problems ran v0.0.7 (About screen screenshot), so each finding says what differs between the two.

## 1. How this was checked

- Read the Rust core (store, engine, providers, FFI), the generated bindings, and the Kotlin app for each flow, from the first tap to the stored row.
- Compared with tag `v0.0.7` wherever behavior differs.
- Spec sources: `docs/product-scope.md` (F1, F5), `docs/build-plan.md` (P1-22, P3-03), `docs/progress-tracker.md`.
- Not available: a crash log (no device is attached to this machine) and an emulator run. Where a claim needs one, it says so. Nothing below is assumed; "proposed" marks design that is not in the code yet.

## 2. The first-run path, link by link

| # | Step | What the code does | v0.0.7 | v0.0.9 (main) |
|---|---|---|---|---|
| 1 | Welcome | Saves an optional name (`OnboardingViewModel.next`) | Works | Same |
| 2 | Add a key | `AddKeyViewModel.saveAndTest` stores the key, then `test_connection`. Only an `Ok` test saves the model list (`provider_settings.rs:318`, `replace_models`) | Works | Same |
| 3 | Open a project | New empty project works. Clone and Import are dead ends (section 3, D3) | Clone and Import open "not built yet" pages | Clone opens the GitHub account screen, not the clone screen. Import still opens a "not built yet" page |
| 4 | New session | `SessionStarter.start`: project default, else app default, else first provider with a key and the first model of its saved list (`SessionStarter.kt:49-70`). No key, no models, and failed each have a banner (`ProjectHomeScreen.kt` `StartBlock`) | Works | Same |
| 5 | Open the session | `SessionRoute` builds a `SessionConnection` and shows `TranscriptScreen` | Plain chat, no tools, so modes change nothing | Opens with the project workspace, tools, and approvals |
| 6 | Send a message | `SessionConnection.send` returns the mentions the core could not attach | Plain streamed reply | Tool loop, approvals, cards |

Spec for this loop (`build-plan.md`, P1-22 done-when): New session picks the first keyed provider and its first model, a typed message streams a reply, Stop works, and a missing key or model list says what to do. The tracker still lists P1-12, P1-13, P1-14, and P1-22 as "In review" with device checks pending, and P1-15 (rename, search) as "In progress, FFI and UI remain" although both exist. No instrumented test covers open, send, rename, or settings changes: `androidTest` holds only `SecretVaultTest` and `NativeLibraryTest`.

## 3. Defects verified in the code

### D1. Bypass is out of sync between core and app (four entry points)

The core and the docs agree: Bypass is biometric, in memory, and off when the session ends (`product-scope.md:136`, `build-plan.md` P3-03). The core enforces it:

- `Store::set_session_mode` and `create_session` return `BypassNeedsConfirmation` for Bypass every time (`tether-store/src/session_load.rs:44-46`, `sessions.rs:62-64`).
- The only way to turn it on is `SessionRuntime.enable_bypass(session_id)` on an open workspace session (`tether-ffi/src/approvals.rs:161`). It flips an in-memory mode and emits only an idle status (`tether-engine/src/session.rs:194-207`). Nothing tells the app that Bypass is on.

The app does not follow that rule:

1. Session mode sheet: after the biometric check it calls `setMode(BYPASS)` (`ModeSheetViewModel.kt` `apply`), which always fails. The error maps to a generic `OTHER` failure (`SessionSettingsGateway.kt:265`, whose own comment says the bypass call "has no screen yet").
2. App default mode: `PermissionsViewModel.kt:88` saves Bypass to preferences. `SessionStarter.kt:86` then calls `setMode(BYPASS)` after creating the session, which throws, so every New session creates an orphan session and reports "could not start".
3. Project default mode: `setProjectDefaults(mode = BYPASS)` returns `BypassNeedsConfirmation` (`projects.rs:205`).
4. `AgentLoopGateway.enableBypass` exists (`AgentLoopGateway.kt:155`) and no screen calls it.

### D2. Thinking effort stops at the provider layer

- All four adapters read `ChatRequest.reasoning`: OpenAI (`openai/request.rs:38`), Anthropic (`anthropic/request.rs:34-39`, a thinking budget), Gemini (`gemini/request.rs:23,34`), and OpenAI-compatible for OpenRouter and plain servers. DeepSeek ignores it (`openai_compat/tests/body.rs:180-181`).
- The engine always sends `reasoning: None` (`tether-engine/src/session/turn.rs:82`, `compact.rs:123`).
- Nothing stores an effort: no session column, no setting, no FFI call, no screen. The model list does carry a per-model reasoning flag (`ModelView.reasoning`, read in Kotlin as `ModelChoice.reasoning`).
- Spec: "Reasoning effort (low, medium, high) appears for models that support it" (`product-scope.md:54`).

### D3. Onboarding dead ends

- Import: `ONBOARDING_ARCHIVE_ROUTE` still shows the placeholder (`OnboardingRoutes.kt:51-57`) although `ImportProjectScreen` exists. Both row subtitles still say "not built yet" (`strings_onboarding.xml:29,31`).
- Clone: the row opens the GitHub account screen. After sign-in the person lands back on the account screen, not on the clone screen.
- The first-run flag is set only by Skip or New empty project (`OnboardingViewModel.finish`). Leaving through Clone or Import keeps the flag unset, so the next launch shows onboarding again.
- "No models" offers only an Open providers button (`ProjectHomeScreen.kt`). The only action that fills the model list is Test connection.

### D4. Blank session rows and hidden rename

- The core already names a session from its first message: `Store::begin_turn` sets the title from the text when the title is empty (80 characters when this was written; since D45 it is at most 60 characters ending at a whole word, with an ellipsis) (`tether-store/src/sessions.rs:149-153`). It does not strip line breaks, so a multi-line first message gives a multi-line title.
- A session that has no message yet has an empty title (`create_session` stores none, `sessions.rs:128-150`). The lists pass the title through unchanged: `CoreHomeSources.kt:64` and `DrawerContent.kt:205` hand `session.title` to `SessionRow`, so such a session is a blank row. `TranscriptScreen` falls back to "Session" only while the snapshot is null, so an empty title also leaves the top bar blank.
- Rename exists only in More, then Rename (`SessionMenuSheet.kt`, `RenameViewModel.kt`). It is not offered from the lists.

### D5. Several runtimes, and recovery runs on every one

- Kotlin builds five `SessionRuntime` objects: `SessionConnection.kt:37`, `SessionGateway.kt:53`, `SettingsGateways.kt:121`, `SessionSheetsModule.kt:15`, `AppModule.kt` (executors). Each builds its own multi-thread Tokio pool (`sessions.rs:104`).
- `SessionRuntime::new` calls `recover_active_sessions`, which sets every session that is not idle, interrupted, or failed to `interrupted` (`tether-store/src/sessions.rs:350-355`). The doc comment says "Reopening it marks unfinished turns as interrupted", which assumes one runtime per process.
- Effect: a runtime created while a turn runs flips that turn's stored status. The practical trigger frequency is not measured.

### D6. Failure paths that can crash the app instead of showing a screen

- `SessionRoute` catches only `SessionRuntimeException` (`TetherNavHost.kt:196`). Any other exception thrown while the connection opens, including a Rust panic that UniFFI raises as an internal exception (`panic = "unwind"`, `core/Cargo.toml:89`), ends the app.
- `KeystoreSecretsPlatform.guarded` catches `GeneralSecurityException` and `IllegalArgumentException` only (`KeystoreSecretsPlatform.kt:86-96`). Keystore failures that are other runtime exceptions escape into the Rust callback.
- `connection.close()` runs on the main thread in `onDispose` (`TetherNavHost.kt:202`) and drops the Rust runtime there.
- No crash recorder exists: no uncaught-exception handler and no crash library (searched `app` and the version catalog).

## 4. What could not be verified: the crash

The crash cause is unknown. No stack trace was available, and static reading did not find one definite bug on the open or send path. These are the code-grounded hazards, in the order I would test them. They are hazards, not findings.

1. D6, first item: any non-`SessionRuntimeException` thrown while opening a session (cause could be a Rust panic).
2. D6, second item: a Keystore runtime exception during the key read at open (`provider_key` in `sessions/open.rs`).
3. D5: a lazily built runtime recovering a running turn.
4. D6, third item: an ANR (not a crash) from dropping the runtime on the main thread.

Step 0 gives the real answer. Run once on the phone (or any device that shows it): `adb logcat -b crash -d > crash.txt`, or copy the red stack from Android Studio Logcat. That names the class and line. Until then, S0 below makes the next crash visible without adb.

## 5. Build order and design

Every slice is built end to end and checked at each hop: core, FFI and bindings, Kotlin gateway, view model, screen. A slice is done only when its hops all pass (section 6).

### S0. Stop the crash, make failures visible (first)

| Hop | Change |
|---|---|
| Core | Recovery leaves `SessionRuntime::new` and becomes one explicit FFI call, `recover_interrupted_sessions(database_path)`, made once from `TetherApp.onCreate` before any runtime exists. A process-wide `Once` would not work: Rust tests share one process, and `reopening_recovers_an_active_turn_and_reuses_its_actor` (`session_tests.rs:193`) expects recovery from the constructor, so that test calls the new function instead. A new test builds two runtimes while a session streams and expects its status to stay |
| Kotlin | `SessionRoute` catches `Exception` and shows `SessionUnavailableScreen` with a reason code; `close()` runs off the main thread on an app scope; `KeystoreSecretsPlatform.guarded` also maps other `RuntimeException`s to `SecretsPlatformFailure.Failed` |
| Kotlin | `TetherApp` installs an uncaught-exception handler that writes the stack to `files/crash/last.txt`; About shows "Last crash" with Copy and Clear |
| Tests | Unit: failure mapping. Instrumented (new `SessionFlowTest` in `androidTest/.../e2e/`): create project and session with a seeded fake key, open with workspace, send, expect a `TurnFailed` event (bad key, no crash), rename, change model and mode, close |
| Done when | The step 0 stack is read and its cause fixed with a test that fails without the fix; opening and closing 20 sessions in a row on a device does not crash |

### S1. First run to first reply

| Hop | Change |
|---|---|
| App | Onboarding Import opens `ImportProjectScreen`; Clone opens `CloneScreen`, and the clone screen's Sign in link returns to it afterwards; both leave the first-run flag set; the "not built yet" strings and the archive placeholder route are deleted |
| App | No models: the banner action runs Test connection for the first keyed provider, then retries; no key: opens Add key |
| App | `SessionStarter` stops leaving an orphan session: set the mode before the session is visible, or delete the session if the mode call fails. Bypass is never read as a default (see S2) |
| Tests | View model tests for each branch; instrumented: fresh install flow with a fake key |
| Done when | On a fresh install: skip nothing, add a key, create a project by each of the three routes, start a session, get a reply, kill the app, reopen the session |

### S2. Session controls that agree with the core

Rename and titles:

- Core: keep the existing rule (title from the first message, set once). One small change: collapse line breaks and runs of spaces so the title is one line, with a test.
- App: rows and the top bar show "New session" for an empty title; Rename is also offered from a long press on a session row (project home and drawer).

Mode and Bypass (follows `product-scope.md:136`):

- Core: add `SessionRuntime.bypass_active(session_id) -> bool` (read of the in-memory mode, FFI, bindings). `enable_bypass` and `disable_bypass` stay. Reopening a runner clears Bypass, and the status line must show that.
- App: choosing Bypass in the mode sheet runs the biometric check, then `AgentLoopGateway.enableBypass`. The status line reads `bypass_active` and shows the red fill. Leaving the session or reopening turns it off. Remove Bypass from the app-default and project-default pickers (D1 items 2 and 3), with the reason shown.
- Other modes: unchanged. They are saved with `setMode` and reopen the runner (`ModeSheetViewModel`).

Model and provider (already built; verify and close gaps):

- The sheet lists providers that have a key and a non-empty saved list (`ModelSheetViewModel.catalog`), then calls `setModel` and `reopen`.
- Verified spec gap: "models without tool support are shown but disabled for agent sessions, with the reason" (`product-scope.md:53`). `ModelSheet.kt` shows the tools, images, and reasoning badges (lines 185-192) and disables rows only while a turn runs (line 70). A model with `tools == false` can be chosen for a workspace session. Build the disabled row with its reason, and keep it enabled for plain chat.

Done when: rename, model, mode, and Bypass each change what the status line and the next turn use, checked by the instrumented test and by hand on a device.

### S3. Thinking effort (new, full chain)

| Hop | Change |
|---|---|
| Types | `ReasoningEffort { Low, Medium, High }` in `tether-types` (the providers crate keeps its own enum; the engine maps one to the other) |
| Store | Migration `0014`: `ALTER TABLE sessions ADD COLUMN reasoning_effort TEXT CHECK (reasoning_effort IN ('low','medium','high'))`, NULL means the model default. `SessionRecord.reasoning_effort`, `set_session_reasoning` (same busy rule as `set_session_model`) |
| Engine | `turn.rs:82` and `compact.rs:123`: send the effort only when the saved model reports reasoning support `Yes`. Unknown or No sends nothing, because OpenAI and Anthropic reject the parameter on models that do not reason |
| FFI | `SessionRecordView.reasoning_effort`, `set_session_reasoning(session_id, Option<ReasoningEffortView>)` calling `forget_runner`, same as model and mode. Regenerate bindings (`just bindings`) |
| Kotlin | `SessionDetails.effort`, `SessionSettingsGateway.setEffort`, a Thinking section in the model sheet (Default, Low, Medium, High). Hidden when the current model's `reasoning` flag is false or unknown, and for DeepSeek, where the adapter ignores effort. App default in Settings > Models, applied by `SessionStarter` like the default mode |
| Tests | Store and engine tests for each provider body; FFI round trip; view model tests; the instrumented test sets effort and checks the stored value |
| Done when | A reasoning model on each of Anthropic, OpenAI, and Gemini sends the matching request field (fixture tests), and the status line or sheet shows the saved level after reopen |

## 6. How end-to-end sync is kept

- One checklist per slice: core test, FFI test, `just bindings` produces no diff (CI job `bindings`), Kotlin gateway fake and view model tests, the instrumented flow test, `just check` (it includes Android lint and the 400-line file limit), then a device check by hand.
- A Rust test that mirrors the exact call order the Kotlin gateways use for each flow, so a mismatch like D1 fails in CI before it reaches a phone.
- The tracker moves a task out of "In review" only after the device check.

## 7. Decisions needed (defaults shown)

1. Automatic title from the first message: already built in the core (80 characters, then 60 on a word boundary since D45). Blair chose to keep it, with rename.
2. Bypass: default per spec (session only, biometric each time). The alternative, a saved Bypass default, would change `product-scope.md:136` and the core rule.
3. Thinking effort default for new sessions: model default (NULL). The alternative is a saved app default level.
4. DeepSeek: hide the control (its adapter ignores effort). The alternative is to make the adapter send it, which needs a provider doc check.

## 8. Not in this scope

Skill install, vault sync, quality settings, status line meters, the diff card, and the remaining backend list (issue #207).

## 9. Status after the build (26 Nov 2026)

Built and checked (`just check` passes, 1,000+ app tests, store, engine, FFI, and providers tests, each new rule mutation-checked):
- S0: one-time recovery, session open failures as a screen, background close, Keystore guard, crash recorder with About card, native `SessionFlowTest` (compiled, not run on a device).
- S1: onboarding Clone and Import, Test connection for no models, no orphan session.
- S2: Bypass inside the session, one-line titles, New session label, rename by long press, models without tools disabled with a reason.
- S3: thinking effort end to end, default Medium, DeepSeek sends it.
- Beyond the original scope, at Blair's request: the composer, top bar, status line, jump pill, plus sheet (with Camera, by decision), and the Listening voice sheet follow the prototype and `docs/ui-rules.md`. The prototype was not edited. The thinking effort control is not in the prototype and was added by decision.
- After the first emulator run: the composer row keeps the send icon in view (the model chip shortens its label), the model sheet has a search field (as in the prototype), the session menu has Rename, Export as Markdown, Checkpoints, Archive with Undo, and Delete with a named confirmation (as in the prototype), the drawer lists archived sessions (chosen by Blair, not in the prototype), and the nav host, drawer, and sheets rise above the keyboard. Not built: swipe to archive on list rows, which the prototype shows.

Not done:
- **The crash cause (section 4) is still unknown.** No stack trace was available. The next crash writes one to About, Last crash.
- `SessionFlowTest` needs a device: `just test-device`.
- Not in the prototype or not built: the context breakdown sheet behind the status line percentage, sending images and files (the core has no image input yet, issue #207), custom commands.
- A model list read on every event is cached per model; a changed catalog shows after the model changes.
