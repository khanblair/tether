# Mobile

Status: draft 0.1, 18 Nov 2026. Tether is a mobile-only product: this doc covers how the Android app works as a platform, and what changes for iOS in v2.

## 1. What the app is and why this approach

Tether is a native Android app: Kotlin and Jetpack Compose for the UI, and a Rust core (`libtether_core.so`) that holds the agent engine, tools, git, sandbox, and storage. It runs on Android 10 and later.

Why native Compose plus a Rust core:

| Need | How the approach meets it |
|---|---|
| Long runs that survive the screen turning off | Direct access to foreground services, wake locks, and notification actions |
| Running code on the phone | Rust hosts Wasmtime in-process and launches PRoot as a child process; neither is practical from a JavaScript or Dart runtime |
| Git with no binary | gitoxide compiles into the same library |
| Fast streaming UI on mid-range phones | Compose with batched updates from the core |
| iOS later without rewriting logic | The same core compiles for iOS; UniFFI generates Swift bindings |

Rejected: React Native and Flutter (an extra runtime between the UI and the core, weaker background service control); Compose Multiplatform for iOS (less native text input and system integration on iOS at the time of the decision).

## 2. How it differs from the other ways to use a coding agent

There is no Tether web version. The useful comparison is with desktop harnesses and with remote-control apps.

| Area | Desktop harness (Claude Code, Codex, OpenCode, Qwen Code) | Remote-control phone apps | Tether |
|---|---|---|---|
| Where the agent runs | On the computer | On the computer | On the phone |
| Needs a computer on | Yes | Yes | No |
| Where code runs | Computer's shell | Computer's shell | WASM sandbox, Linux mode, or GitHub Actions |
| Models | One vendor (Claude Code, Qwen Code) or many (OpenCode) | Whatever the desktop tool supports | Five providers, any of their tool-capable models |
| Input | Keyboard | Touch | Touch, voice, share sheet, hardware keyboard |
| Approvals | In the terminal | In the app | In the app and from notifications |
| Diff review | Terminal or IDE | Small-screen view of terminal output | Purpose-built mobile diff view |

## 3. Connecting and pairing

There is no pairing. Tether never connects to a PC, and there is no Tether account or server. Connections are only to services the user signs into:

| Service | How the user connects | Stored as |
|---|---|---|
| Model providers | Paste an API key in Settings → Providers; **Test connection** confirms it | Keystore-wrapped secret |
| GitHub | OAuth device flow: Tether shows a code and opens the GitHub page; or paste a fine-grained token | Keystore-wrapped secret |
| MCP servers | Add by URL with optional header or OAuth | Keystore-wrapped secret for credentials |
| Telegram (v1.1), Discord (v1.2) | Paste the bot token the user created; Tether shows the steps | Keystore-wrapped secret |
| Google Workspace (v1.3) | Google sign-in with per-feature scopes | Token in Keystore-wrapped storage |

Switching phones: export a project as a zip, or push it to GitHub and clone on the new phone. Keys are not exported; the user adds them again.

## 4. Offline behavior

| Feature | Offline |
|---|---|
| Browse projects, sessions, transcripts, diffs, files | Works |
| Rewind | Works |
| Manual `/run` in the sandbox | Works if the runtime pack is installed |
| Manual run in Linux mode | Works for commands that need no network |
| Git commit and branch | Works |
| Agent turns | Need the provider; a running turn pauses and resumes on reconnect |
| Push, PR, CI, clone, web tools, MCP | Need the network; shown as unavailable with the reason |
| New runtime pack or Linux setup | Need the network |

The offline banner appears under the status line. Turns never fail only because the network dropped; they pause (architecture.md, section 6).

## 5. Native features

| Feature | Android API | Used for |
|---|---|---|
| Foreground service | `ForegroundService` with a declared type (see open question M-Q1) | Keep turns running in the background |
| Partial wake lock | `PowerManager.PARTIAL_WAKE_LOCK`, held only during an active turn, released on finish, 30-minute timeout renewed per tool call | Keep CPU awake for long tool runs with the screen off |
| Notifications with actions | `NotificationCompat` with `PendingIntent`s | Approvals, run status |
| Keystore | AES-GCM key, StrongBox when available | Wrap secrets |
| Biometric | Platform `BiometricPrompt` (`android.hardware.biometrics`). Biometric, and from Android 11 also the screen lock (PIN, pattern, or password), so a phone with no enrolled fingerprint still works | Reveal keys, Bypass, app lock |
| Speech input | `SpeechRecognizer` | Voice prompts |
| Photo picker and camera intent | `PickVisualMedia`, `ACTION_IMAGE_CAPTURE` | Image attachments, no storage permission needed |
| Storage Access Framework | `OpenDocument`, `CreateDocument`, `OpenDocumentTree` | Zip import and export; the shared folder for vault sync (the Kotlin layer copies files, because the Rust core cannot open content URIs) |
| Adaptive and themed launcher icon | Adaptive icon with a vector foreground, `mint-100` background, and a monochrome layer | Logo on every launcher; tinted on Android 13 and later |
| Window and splash colors | `Theme.Tether` with the `bg` token as window and splash background | No white flash in the dark theme |
| Share sheet intake | `ACTION_SEND` text/plain filter; project picker is staged | Session creation and file shares remain |
| App shortcuts | Static and dynamic shortcuts | Jump to the last session; new session in a recent project |
| Screen capture blocking | `FLAG_SECURE`, optional | Privacy setting |
| Battery settings intent | `ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS` | Guidance screen; we do not request the restricted direct-exemption permission |

### Manifest permissions

Status: built for the permissions below. The manifest declares `INTERNET`, `ACCESS_NETWORK_STATE`, `USE_BIOMETRIC`, `POST_NOTIFICATIONS`, and `RECORD_AUDIO`.

| Permission | Why | When asked |
|---|---|---|
| `INTERNET`, `ACCESS_NETWORK_STATE` | Providers, GitHub, downloads | Install time |
| `POST_NOTIFICATIONS` (Android 13+) | Approvals and run status | First time a turn runs in the background |
| `FOREGROUND_SERVICE` plus the type-specific permission | Long runs | Install time |
| `WAKE_LOCK` | Partial wake lock during turns | Install time |
| `USE_BIOMETRIC` | Key reveal, Bypass, app lock | Install time |
| `REQUEST_INSTALL_PACKAGES` (`full` only) | Install a downloaded update through `PackageInstaller` (P1-20); `play` never declares it | First tap on Install update, through the system settings screen |
| `RECORD_AUDIO` | Voice input in the composer. The device speech recognizer does the work; Tether records and stores no audio | First tap on the mic in the composer |

Voice input shows a "Listening" sheet that fills the composer with the words it hears. Nothing is sent automatically.

No location, contacts, or broad storage permissions.

## 6. Notifications

| Channel | Importance | Content | Actions |
|---|---|---|---|
| Approvals | High (heads-up) | "Run in sandbox: `pytest -q`" or "Edit 3 files in app/" | Allow, Deny, Open |
| Agent running | Low (silent, ongoing) | Project, current step, elapsed time | Interrupt, Open |
| Run results | Default | "Session finished: 4 files changed, tests passing" or the failure | Open, Review diff |
| CI | Default | "CI failed on tether/fix-login: 2 tests" | Open |
| Linux mode setup | Low | Download and install progress | Cancel |

Rules:
- One ongoing notification per running session; it is updated in place, never stacked.
- Approval notifications for edits show a summary only; full diffs need the app.
- Notification text follows ui-rules.md writing rules; no key material, no file contents beyond a 1-line command.
- On the lock screen, approval notifications show only the project name and "Approval needed" unless the user allows full content in Settings.

## 7. Mobile screens and gestures

Screens, in navigation order: Onboarding → Project list (drawer) → Project home → Session → Diff, Files, Runs, Linux shell → Settings → Cost.

Gestures, each with a visible alternative (ui-rules.md section 5):

| Gesture | Where | Alternative |
|---|---|---|
| Edge swipe from left | Opens the project drawer | Menu button in the top bar |
| Long press | Message menu | Overflow button on hover or focus |
| Swipe left | Archive a session row | Row overflow menu |
| Pull down | Load older messages | Automatic when scrolled to top |
| Drag sheet handle | Resize bottom sheets | Tap handle to toggle |
| Pinch | Text size in the diff and file viewers | View menu: smaller, larger |

Predictive back is supported. Back from a running session leaves the turn running.

## 8. Design rules for mobile

- Primary actions in the bottom 40% of the screen.
- The composer and approval bar stay above the keyboard; the transcript keeps its scroll anchor when the keyboard opens.
- Edge-to-edge display with system bar insets applied to the top bar and composer.
- Dense information in monospace captions, but never below 12 sp.
- Horizontal orientation on phones: transcript only, composer collapses to one line until focused.
- Foldables and tablets use the layouts in ui-rules.md section 10.
- Dark mode follows the system by default; the setting overrides it. The palette is Velvet Plum (ui-tokens.md). Material dynamic color is off, so wallpaper colors never replace it.

## 9. Security

The full model is in architecture.md section 10. Mobile-specific points:

- Secrets are wrapped with a Keystore key that requires no user authentication for use (so background turns can call providers), and are revealed in the UI only after biometric authentication.
- App lock (optional) requires biometric or device credential when the app comes to the foreground after 5 minutes away.
- Backups exclude secrets, the Linux root filesystem, runtime packs, and by default workspaces (`backup_rules.xml`, `data_extraction_rules.xml`). Memory, skills, and the vault are included in backups; they hold no secrets and writes are scanned (architecture.md section 14).
- Network security config forbids cleartext traffic.
- Exported components: only the main activity, the share intent filter, and the notification action receiver (which accepts only our signed `PendingIntent`s).
- The `full` flavor's PRoot binaries live in `nativeLibraryDir` and are signed as part of the APK. The Alpine root filesystem is downloaded and verified against the signed manifest.

## 10. Budgets

| Budget | Target |
|---|---|
| Cold start to project list | p95 under 1.5 s on the reference device |
| APK size, `play`, per ABI | Under 35 MB |
| APK size, `full`, per ABI | Under 38 MB (adds PRoot) |
| Runtime packs | Python under 30 MB, JavaScript under 3 MB, SQLite under 2 MB compressed |
| Linux mode disk use after setup | Under 250 MB with base packages |
| Memory, excluding Linux mode | Under 300 MB |
| Battery, active session, screen off | Under 8% per hour |
| Idle background | No wake locks, no network, no services when no turn is running |
| Mobile data, idle | Zero |

## 11. Code structure

The module layout is in project-structure.md. Mobile-specific placement:

- `service/`: `AgentService`, notification channels and builders, `ApprovalReceiver`.
- `platform/`: biometric, speech, file picker, share intake.
- `core-bridge/`: the only module that talks to the Rust core; implements `SecretsPlatform`, `NotifyPlatform`, `NetworkPlatform`.
- `app/src/full/`: Linux mode components and PRoot `jniLibs`; `app/src/play/`: the Linux mode stub.

## 12. Development

Setup, commands, and troubleshooting are in development.md. For mobile work specifically:

- Test background behavior on physical devices only: one Samsung, one Pixel, one Xiaomi or Oppo, plus the reference device.
- Use `adb shell dumpsys deviceidle force-idle` to test Doze, and the Developer menu's "Kill process during next tool" for process death.
- Use `adb shell cmd notification` and Android Studio's App Inspection to check notifications and the database.

## 13. Release

Release steps are in development.md section 13. Mobile-specific items:

- Google Play: Data safety form states that no data is collected by Tether; data goes to providers and GitHub at the user's direction. Foreground service type declaration and justification. Policy decision on Linux mode (product-scope.md Q1).
- GitHub Releases (`full`): one universal APK now (split APKs per ABI plan for P10), a `SHA256SUMS` file attached to the release, and in-app updates (P1-20): the app checks the latest release, and the user can download, verify against `SHA256SUMS`, and install it from Settings > About > Updates. The system installer asks the user to confirm, and the new APK must carry the same signing certificate as the installed app. The app checks when it starts (at most every 12 hours) and whenever About opens, and shows a dot on the bell and an Updates item in Needs attention when a newer version exists. The first release that contains the updater has to be installed by hand once; later releases arrive through it. Every merge to `main` that changes code publishes a release (D34), so updates arrive often.
- Staged rollout on Play: 10%, 50%, 100% over a week, watching locally exported crash logs from beta testers.

## 14. Tasks

Mobile-specific tasks from build-plan.md:

| ID | Task |
|---|---|
| P0-05 | cargo-ndk build into jniLibs |
| P1-03 | Keystore secrets |
| P1-17 | Voice input |
| P1-20 | In-app updates for the `full` build |
| P3-03 | Bypass with biometric |
| P7-01 | PRoot as `lib*.so` from `nativeLibraryDir` |
| P7-09 | `play` flavor stub |
| P9-01 | Foreground service |
| P9-02 | Approval notifications with actions |
| P9-03 | Process-death recovery |
| P9-09 | OEM battery guidance |
| P9-10 | Share sheet intake |
| P10-02 | Accessibility pass |
| P10-09 | Store listing and data safety |

## 15. Open questions

| ID | Question | Default until decided |
|---|---|---|
| M-Q1 | Foreground service type: `dataSync` (6-hour daily cap on Android 15+) or `specialUse` (needs Play justification)? | `specialUse` in `full`; decide for `play` during P10 review |
| M-Q2 | Offer an in-app update download for `full`, or only notify? | Decided (D22): the `full` build downloads, verifies, and installs updates from Settings > About > Updates; `play` links to Google Play |
| M-Q3 | Support Android 9 (API 28) for older devices? | No; Android 10+ |
| M-Q4 | Ship x86_64 in the Play bundle or emulator builds only? | Ship both ABIs |
| M-Q5 | Keep the vault in app storage with zip and folder sync, or in shared storage so Obsidian opens it directly? | App storage plus sync (product-scope Q9) |

## 16. iOS plan (v2)

| Area | Android v1 | iOS v2 |
|---|---|---|
| UI | Jetpack Compose | SwiftUI over UniFFI Swift bindings |
| Core | `libtether_core.so` | Same Rust core as a static library in an XCFramework |
| Sandbox | Wasmtime with AOT compilation | Wasmtime's interpreter mode (iOS forbids JIT); slower, same isolation and runtimes |
| Linux mode | PRoot + Alpine (`full`) | Not available; iOS does not allow launching processes |
| Heavy work | GitHub Actions | GitHub Actions, used more often |
| Background runs | Foreground service | Limited: background tasks get short windows; long turns continue when the user returns, with Live Activities showing progress |
| Approvals from notifications | Notification actions | Notification actions through `UNNotificationAction` |
| Secrets | Android Keystore | iOS Keychain with Secure Enclave key wrapping |
| Distribution | Play and GitHub Releases | App Store only |

Work to do now so iOS is cheap later:
- Keep all logic in the core; Kotlin holds only UI and platform glue.
- Keep platform callbacks (`SecretsPlatform`, `NotifyPlatform`, `NetworkPlatform`) small and free of Android types.
- Generate tokens for Swift from the same `tokens.json`.
- Design the turn engine so a paused turn can resume cleanly, since iOS will pause more often.
