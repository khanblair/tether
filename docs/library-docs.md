# Library docs

Status: draft 0.2, 18 Nov 2026. Only libraries listed here may be used. Exact versions are pinned in `core/Cargo.lock` and `android/gradle/libs.versions.toml`; the Version column gives the series to use.

## 1. Rules for adding a library

A new library needs a pull request that adds its row to this doc, and meets all of these:

1. It solves a problem we have now, not one we might have.
2. Writing it ourselves would take more than a day, or would be security-sensitive (crypto, parsing untrusted formats).
3. It had a release or meaningful commit in the last 12 months, or is stable and widely used.
4. License is MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, or OFL (fonts). No GPL, LGPL, or AGPL in the app. `cargo-deny` enforces this for Rust.
5. It adds no analytics, ads, or network calls of its own.
6. Its size cost is stated in the PR: added APK bytes per ABI for Android, or added `.so` bytes for Rust.
7. It is not on the banned list in section 5.

AI agents do not add libraries. They propose them in the PR description and stop (code-standards.md section 13).

## 2. Rust core

### Runtime and networking

| Library | Version | Use | Reason |
|---|---|---|---|
| `tokio` | 1.x | Async runtime, channels, timers, blocking pool | Standard; needed by reqwest and our actors |
| `tokio-util` | 0.7.x | Cancellation tokens, codecs | Clean turn interruption |
| `reqwest` | 0.12.x, features `rustls-tls`, `stream`, `json`; default features off | All HTTP | One client for providers, GitHub, MCP, downloads |
| `rustls-platform-verifier` | Latest | Use the OS trust store on Android and iOS | Matches system certificates, including user-installed ones where Android allows |
| `url` | 2.x | URL parsing | Correct handling for web_fetch and MCP |

### Data and serialization

| Library | Version | Use | Reason |
|---|---|---|---|
| `serde`, `serde_json` | 1.x | All JSON | Standard |
| `toml` | 0.8.x or later | `.tether/config.toml` | Standard TOML parser |
| `schemars` | Latest 0.x or 1.x | Generate tool JSON schemas from Rust types | Schemas and code cannot drift |
| `jsonschema` | Latest | Validate model-supplied tool arguments | Models send malformed arguments; reject early |
| `ulid` | 1.x | IDs | Sortable, text-friendly |
| `thiserror` | 2.x | Error enums | Clear typed errors |

### Storage and files

| Library | Version | Use | Reason |
|---|---|---|---|
| `rusqlite` | Latest, feature `bundled` | SQLite | Bundled SQLite gives the same version on every device |
| `zeroize` | 1.x | Wipe secret bytes when they are dropped | The standard way to clear memory so the compiler cannot optimize it away. Added by P1-03; the owner approves it when merging that pull request (code-standards 13.4) |
| `blake3` | 1.x | Blob hashing | Fast content addressing |
| `zstd` | Latest | Blob compression | Good ratio and speed on phones |
| `ignore` | Latest | Walk files with `.gitignore` rules | From ripgrep; fast and correct |
| `globset` | Latest | Glob tool | Same family as `ignore` |
| `grep-searcher`, `grep-regex` | Latest | Grep tool | ripgrep's engine as a library |
| `notify` | Latest | File watcher for the index | Cross-platform watcher |
| `similar` | 2.x | Diffs and hunks | Clean API for unified diffs |

### Git and GitHub

| Library | Version | Use | Reason |
|---|---|---|---|
| `gix` (gitoxide) | Pinned 0.x, features for blocking HTTP transport with reqwest | Clone, status, diff, commit, push | Pure Rust; no C library to cross-compile |

GitHub REST and GraphQL calls use `reqwest` directly with typed request and response structs. No GitHub SDK crate.

### Execution

| Library | Version | Use | Reason |
|---|---|---|---|
| `wasmtime` | Pinned major (monthly releases; upgrade quarterly) | Sandbox engine | Mature, epoch interruption, AOT precompile, interpreter mode for iOS |
| `wasmtime-wasi` | Same as wasmtime | WASI for sandbox runs | Matches the engine |
| `tar` | 0.4.x | Extract root filesystem and packs | Standard |
| `flate2` | 1.x | Gzip for the Alpine tarball | Standard |
| `ed25519-dalek` | 2.x | Verify manifest signatures | Audited, pure Rust |
| `sha2` | 0.10.x or later | Verify download hashes | Matches published Alpine checksums |
| `vt100` | Latest | Parse terminal output for the `/linux` shell | Rendering stays portable to iOS |

### Quality and text

| Library | Version | Use | Reason |
|---|---|---|---|
| `tree-sitter` | Pinned 0.x | Parsing for the quality gate and highlighting | Fast, incremental, many grammars |
| `tree-sitter-highlight` | Same series | Syntax highlighting spans for the UI | One highlighter for Android and iOS |
| Grammar crates (`tree-sitter-python`, `-javascript`, `-typescript`, `-rust`, `-go`, `-kotlin-ng` or maintained equivalent, `-java`, `-swift`, `-c`, `-cpp`, `-ruby`, `-php`, `-html`, `-css`, `-json`, `-yaml`, `-toml-ng`, `-md`, `-bash`) | Compatible with the pinned `tree-sitter` | Languages listed in architecture.md | Pin each to a version built for the same ABI |
| `pulldown-cmark` | Latest | Parse Markdown into blocks sent to the UI | Android and iOS render the same structure |
| `regex` | 1.x | Slop patterns, failure parsing | Standard |
| `html2text` | Latest | Readable text for `web_fetch` | Small, no browser engine |

### Observability and FFI

| Library | Version | Use | Reason |
|---|---|---|---|
| `tracing` | 0.1.x | Structured logs | Standard |
| `tracing-subscriber` | 0.3.x | Log filtering and formatting | Standard |
| `tracing-appender` | 0.2.x | Rotating log files | 7-day rotation |
| `uniffi` | 0.32.x (proc-macro and library mode; the `cli` feature only for the generator binary) | Kotlin bindings now, Swift later | Decision D3 |

### Dev and test only

| Library | Use |
|---|---|
| `criterion` | Benchmarks in `core/benches` |
| `proptest` | Property tests for the path guard and edit tools |
| `insta` | Snapshot tests for prompt assembly and diff output |
| `wiremock` | HTTP mocks for GitHub and MCP tests |
| `tempfile` | Temporary workspaces in tests |

## 3. Android

| Library | Version | Use | Reason |
|---|---|---|---|
| Jetpack Compose BOM | Latest stable | All UI | Decision D2 |
| Material 3 (Compose) | From BOM | Base components we restyle with tokens | Accessibility and input behavior done right |
| Navigation Compose | 2.10.x. P0 uses string routes (`nav/Route.kt`); type-safe routes need kotlinx-serialization and arrive with the first route that takes arguments | Navigation | Official |
| Lifecycle (`viewmodel-compose`, `runtime-compose`) | Latest stable | ViewModels, `collectAsStateWithLifecycle` | Official |
| Hilt and `hilt-navigation-compose` | Latest stable | Dependency injection. Planned: P0 uses a hand-wired `AppModule` | Compile-time checked |
| kotlinx-coroutines | 1.11.x | Async; `SharedFlow` for core events | Standard |
| kotlinx-serialization-json | 1.x | Navigation arguments and small UI-side data | Official; no reflection |
| DataStore Preferences | Latest stable | UI-only preferences (theme override, onboarding seen). Planned: P0 keeps the theme override in memory | Everything else lives in the core |
| JNA (`@aar`) | 5.19.x | Runtime for UniFFI Kotlin bindings | Required by UniFFI |
| AndroidX Biometric | Latest stable | Key reveal, Bypass, app lock | Official |
| AndroidX Core and Activity Compose | Core 1.19.x, Activity 1.13.x | `NotificationCompat`, `ServiceCompat`, edge-to-edge | Official |
| AndroidX ProfileInstaller | Latest stable | Baseline profiles for startup | Cold start budget |

Fonts: IBM Plex Sans (Regular, Medium, SemiBold) and IBM Plex Mono (Regular, Medium), SIL Open Font License, bundled in `designsystem/src/main/res/font/` with the license in `designsystem/FONTS-LICENSE.txt`.

Icons: Lucide (ISC), converted to Compose `ImageVector`s in `designsystem/Icons.kt`. Only icons in use are converted.

### Build and quality tooling

| Tool | Version | Use |
|---|---|---|
| Android Gradle Plugin | 9.5.x (built-in Kotlin) | Android builds |
| Kotlin and Compose compiler plugin | 2.4.x | Language and Compose |
| Gradle | 9.6.1, pinned with a checksum in the wrapper | Build |
| detekt | 1.23.x | Kotlin static analysis |
| ktlint-gradle | 14.x | Kotlin formatting |
| cargo-ndk | 4.x | Rust to Android `.so` files |
| cargo-deny | Latest | Rust licenses and advisories |
| marked | 18.x | Docs site: Markdown to HTML (Node, dev only) |
| `@resvg/resvg-js` | 2.x | Icon export: SVG to PNG (Node, dev only) |
| `regex`, `serde`, `toml` | 1.x, 1.x, 0.9.x | `tools/slop-lint` only |

### Android test only

| Library | Use |
|---|---|
| JUnit 4 | Unit and instrumented test runner (Compose testing requires it) |
| AndroidX Test (`ext:junit`, `runner`) | Instrumented tests; `NativeLibraryTest` runs on a device or emulator |
| kotlinx-coroutines-test | Test dispatchers |
| Turbine | Testing `Flow`s |
| Compose UI Test | UI tests |
| Robolectric and Roborazzi | Screenshot tests on the JVM |
| AndroidX Macrobenchmark | Startup and scroll benchmarks |
| MockK | Only for platform classes that cannot be faked otherwise |

## 4. Usage notes and gotchas

- **reqwest:** one shared `Client` in the engine, created with timeouts (connect 10 s, read 120 s for streams). Never create a client per request; it defeats connection reuse.
- **SSE streams:** providers split events across TCP chunks and sometimes send comments or empty lines. `tether-providers/sse.rs` handles this; do not parse SSE elsewhere.
- **rusqlite:** bundled SQLite compiles C code, so Android cross-checks go through `cargo ndk` (CI does this). One write connection owned by the store, used from a blocking thread. Read connections may run in parallel thanks to WAL. Never hold a transaction across an `.await`.
- **gix:** feature flags are fine-grained; enable only what we use. Push over HTTPS uses our credential callback with the GitHub token; never embed the token in the URL (it would land in config and logs).
- **wasmtime:** create one `Engine` per process. Precompiled `.cwasm` files are only valid for the same Wasmtime version and engine config; the cache key includes both. Use epoch interruption for time limits, not fuel (fuel is slower).
- **WASI mounts:** preopen only `/workspace` and a per-run temp dir. Do not inherit the host environment.
- **PRoot binaries:** must be named `lib*.so` and placed in `jniLibs` with `useLegacyPackaging = true` so Android extracts them to `nativeLibraryDir`, the only place an app may execute files from on Android 10+.
- **tree-sitter:** grammars must match the core library ABI version. Upgrade the core and all grammars in one PR.
- **UniFFI:** callback interfaces are called from Rust threads. Kotlin implementations must be thread-safe and must not touch UI directly.
- **JNA and R8:** keep rules in `core-bridge/consumer-rules.pro` (and `app/proguard-rules.pro`) are required or release builds crash at first core call.
- **Compose and streaming text:** append-only text should be a single `AnnotatedString` per block, updated in place; do not create a new list item per delta.
- **Material 3:** use its components for behavior (text fields, sheets, menus), but colors, shapes, and type always come from Tether tokens, never `MaterialTheme` defaults. `TetherTheme` switches dynamic color off; do not turn it on.
- **Convention plugins:** `cargo ndk` must be on the Gradle process `PATH`. `CargoNdkTask` adds `~/.cargo/bin`, so builds from Android Studio work without a login shell.

## 5. Banned libraries

| Banned | Why | Use instead |
|---|---|---|
| OkHttp, Retrofit, Ktor client in the app | Network logic belongs in the core | `reqwest` in Rust |
| Gson, Moshi | Reflection, duplicates kotlinx-serialization | kotlinx-serialization |
| LiveData | Lifecycle-coupled and legacy for this stack | `StateFlow` |
| RxJava | Second async model | Coroutines and `Flow` |
| Room | Second database | SQLite in the core |
| AndroidX Security Crypto (`EncryptedSharedPreferences`) | Deprecated | Keystore wrapping through `SecretsPlatform` |
| `git2` / libgit2 | C dependency, cross-compile pain | `gix` |
| `openssl`, `native-tls` | C dependency, inconsistent on Android | `rustls` |
| `anyhow` in library crates | Erases error types at boundaries | `thiserror` |
| `lazy_static`, `once_cell` | Replaced by the standard library | `std::sync::LazyLock`, `OnceLock` |
| Firebase Analytics, Crashlytics, any analytics SDK | No data leaves the device without consent (Q3) | Local crash log export |
| Termux packages or its app code | GPL licensing; we own the Linux layer | Our PRoot build and Alpine |
| Accompanist modules with official replacements | Deprecated | Official AndroidX APIs |
| Compose Multiplatform for app UI | Conflicts with decision D2 | Jetpack Compose now, SwiftUI later |
| Any WebView-based UI for core screens | Slow, inconsistent | Compose |

## 6. Open library decisions

| Area | Options | Leaning | Decide by |
|---|---|---|---|
| Logcat output from `tracing` | `paranoid-android`, a small custom layer over `android_log-sys` | Custom layer if the crate is unmaintained | P1 (the core has no logging yet) |
| Charts for `/cost` | Vico, custom Canvas | Custom Canvas (two chart types only) | P9 |
| Python runtime pack source | CPython official WASI builds, a community WASI distribution | Official CPython WASI build | P5 |
| JavaScript in the sandbox | QuickJS (WASI build), a Node-compatible WASI runtime | QuickJS now; revisit when a stable Node-compatible option exists | P5 |
| HTML to text | `html2text`, a readability port | `html2text`; add readability if fetch results are noisy | P9 |
| MCP client | Write our own, official Rust MCP SDK | Official Rust SDK if it supports Streamable HTTP client without extra runtimes | P9 |
| Vault Markdown and YAML frontmatter parsing | `pulldown-cmark` with a maintained YAML crate; a small hand-written parser | `pulldown-cmark` for Markdown, a small frontmatter reader, own wikilink parser | P11 |
| Skill static scan | Pattern rules shared with `tether-quality`, a dedicated scanner | Pattern rules in `tether-quality` data files | P11 |
