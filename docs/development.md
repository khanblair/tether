# Development

Status: draft 0.2, 18 Nov 2026.

Tether users need no PC. Building Tether itself does: development happens on macOS, Linux, or Windows with WSL2.

## 1. Required tools

| Tool | Version | Why |
|---|---|---|
| rustup and Rust stable | Pinned in `rust-toolchain.toml` | Core build |
| Android Rust targets | `aarch64-linux-android`, `x86_64-linux-android` (installed by the toolchain file) | Cross-compiling the core |
| cargo-ndk | 4.x (tested with 4.1.2) | Builds `.so` files per ABI |
| Android Studio | Latest stable | IDE, SDK manager, emulator, profiler. Optional: the command line works without it |
| JDK | 21 (17 or newer works; CI uses 21) | Runs Gradle. Code compiles to Java 17 bytecode |
| Android SDK | Platform 37 (`compileSdk`), build tools 36.0.0, platform tools | App build. AndroidX libraries in the version catalog need `compileSdk` 37 |
| Android NDK | 27.1.12297006, pinned in `android/gradle.properties` as `tether.ndkVersion` | Native build |
| just | 1.x | Command runner |
| Node.js | 22.18 or later (24 in CI) | Runs the TypeScript tooling directly: token generator, icon export, docs site |
| pnpm | 11 (`packageManager` in `package.json`) | Installs the Node tooling from `pnpm-lock.yaml` |
| cargo-deny | Latest | License and advisory checks |
| An emulator or a physical Android device | Android 10+. Apple silicon Macs run arm64 emulators; Intel machines and CI use x86_64 | Running the app. A physical device, ideally the reference device, is needed for background, battery, and Linux mode testing |

## 2. First-time setup

```
git clone https://github.com/khanblair/tether.git
cd tether
just setup          # checks tools, installs the pinned Rust toolchain, cargo-ndk, cargo-deny, and Node packages
just run            # build fullDebug, install it on the connected device or emulator, and start it
```

Tokens and bindings are committed, so a fresh clone builds without `just tokens` or `just bindings`. Run those two only after changing `tokens/tokens.json` or the Rust FFI surface.

`.env` is not needed until provider tests exist (P1). When it is, copy `.env.example` to `.env` and add keys (section 5).

`just setup` stops with a clear message if the NDK in `gradle.properties` is missing. Install it with `sdkmanager "ndk;27.1.12297006"` or from Android Studio's SDK Manager under SDK Tools, "NDK (Side by side)".

To run on an emulator: start one (`emulator -avd <name>`), then `just run`. P0 was verified on a Pixel 9 arm64 emulator.

To run from Android Studio, open `android/`, pick the `fullDebug` build variant, and press Run. A debug APK holds the core for both ABIs and is about 360 MB, which can fill a small emulator (`INSTALL_FAILED_INSUFFICIENT_STORAGE`). On an arm64 emulator, build one ABI: add `tether.abis=arm64-v8a` to `~/.gradle/gradle.properties` (not to the repo's `gradle.properties`). The debug APK drops to about 135 MB and the Rust build time halves. CI and releases never set it, so they build both ABIs.

## 3. Workspace layout

See project-structure.md. Day to day you will mostly touch `core/crates/tether-engine`, a provider or tool crate, and one `android/feature-*` module.

Open `android/` in Android Studio. Open the repo root in a Rust-aware editor (RustRover or VS Code with rust-analyzer) for the core. Gradle builds the core automatically through the `tether.rust-core` convention plugin (`CargoNdkTask`), so one Run in Android Studio builds both.

## 4. Dev mode

Status: planned (P1 and later). Debug builds will add a **Developer** section at the bottom of Settings:

| Option | What it does |
|---|---|
| Fake provider | Scripted model that replays a YAML script from `core/fixtures/scripts/`; no network, no cost |
| Event log | Live list of `CoreEvent`s with timestamps |
| Kill process during next tool | Tests process-death recovery |
| Network simulator | Adds latency, drops, or offline periods to core HTTP |
| Show recomposition counts | Overlay for Compose performance work |
| Reset onboarding | Shows first-run screens again |
| Open data folder | Shares a zip of `files/` minus `secrets/` for debugging |

## 5. Environment variables

Stored in `.env` at the repo root, read by `just` recipes and test runners. Never committed.

| Variable | Used by | Purpose |
|---|---|---|
| `TETHER_TEST_ANTHROPIC_KEY` | Live smoke tests | Anthropic key for cheap-model tests |
| `TETHER_TEST_OPENAI_KEY` | Live smoke tests | OpenAI key |
| `TETHER_TEST_GEMINI_KEY` | Live smoke tests | Gemini key |
| `TETHER_TEST_DEEPSEEK_KEY` | Live smoke tests | DeepSeek key |
| `TETHER_TEST_OPENROUTER_KEY` | Live smoke tests | OpenRouter key |
| `TETHER_TEST_GITHUB_TOKEN` | GitHub tests | Fine-grained token for the test repo |
| `TETHER_TEST_REPO` | GitHub and CI tests | `owner/name` of a dedicated test repo |
| `TETHER_GITHUB_CLIENT_ID` | Build | OAuth app client ID for the device flow (public, not secret) |
| `TETHER_KEYSTORE_PATH` | Release builds | Path to the release keystore |
| `TETHER_KEYSTORE_PASSWORD` | Release builds | Keystore password |
| `TETHER_KEY_ALIAS` | Release builds | Signing key alias |
| `TETHER_KEY_PASSWORD` | Release builds | Signing key password |
| `TETHER_MANIFEST_KEY` | Pack and rootfs releases | Ed25519 private key for signing manifests |

In CI these come from GitHub Actions secrets. The release workflow reads the keystore as a base64 string, so its secret has a different name from the local variable:

| Secret | Local equivalent |
|---|---|
| `TETHER_KEYSTORE_BASE64` | The file at `TETHER_KEYSTORE_PATH`, encoded with `base64` |
| `TETHER_KEYSTORE_PASSWORD`, `TETHER_KEY_ALIAS`, `TETHER_KEY_PASSWORD` | Same names |

Only the four signing variables are used today. The provider and GitHub test variables are used from P1 and P4.

## 6. Real services in development

- Use the fake provider for UI work and engine tests. Use real providers only for adapter work and smoke tests.
- Smoke tests use the cheapest model of each provider and cap output at 64 tokens.
- GitHub work uses the dedicated test repo in `TETHER_TEST_REPO`, never a personal or production repo.
- Linux mode needs a real download of the Alpine root filesystem; use Wi-Fi.
- Keep a monthly spend limit on every test key at the provider's dashboard.

## 7. Everyday commands

| Command | What it does |
|---|---|
| `just setup` | Check tools; install the pinned toolchain, cargo-ndk, cargo-deny, and Node packages |
| `just run` | Build and install `fullDebug` on the connected device or emulator, then start it |
| `just run-play` | Same for `playDebug` |
| `just core` | `cargo build` for host |
| `just test` | All Rust, Kotlin, and tooling unit tests |
| `just test-core` | `cargo test --workspace` |
| `just test-android` | Gradle unit tests for all modules |
| `just test-tools` | `slop-lint` tests |
| `just test-device` | Instrumented tests on the connected device or emulator |
| `just smoke` | Live provider smoke tests (planned, P1) |
| `just lint` | rustfmt, clippy, ktlint, detekt, Android lint, token check |
| `just slop` | `tools/slop-lint` over docs, README, and `strings.xml` |
| `just smell-fixtures` | Prove Clippy and detekt fail on a function over the 50 line limit |
| `just commits` | Fail if a commit on this branch touches more than 15 files (compares with `origin/main`) |
| `just smells` | File length script and its self test. Complexity and length thresholds run inside `just lint` through Clippy and detekt |
| `just bindings` | Regenerate UniFFI Kotlin bindings |
| `just tokens` | Regenerate `Tokens.kt` and the Android color resources from `tokens/tokens.json`; fails if a contrast pair is too low |
| `just icons` | Render PNG brand assets from `images/logo.svg` |
| `just prototype` | Serve the mobile UI prototype at http://127.0.0.1:4173/ |
| `just site` | Build the docs site into `site-dist/` |
| `just bench` | Core benchmarks (planned) |
| `just packs` | Build sandbox runtime packs locally (planned, P5) |
| `just linux-libs` | Build PRoot libraries for both ABIs (planned, P7) |
| `just check` | `lint`, `slop`, `smells`, `smell-fixtures`, `test`, and a bindings diff check: run before every push |
| `just release-signing` | One time: create the release keystore and store it as the four GitHub secrets (run it in your own terminal) |
| `just release` | Build `assembleFullRelease` and `bundlePlayRelease` (needs the signing variables) |

## 8. Testing notes

- Engine tests use the fake provider. Write a script in `core/fixtures/scripts/` for each scenario.
- Record new provider streams with `just record-stream <provider> <case>`; it saves SSE bytes with keys stripped. (Planned, P1.)
- Compose screenshot tests use Roborazzi. Update goldens with `just screenshots-record` only when the change is intended, and say so in the PR. (Planned, P1-19.)
- Background behavior cannot be tested on the emulator reliably. Use a physical device and the protocol below.

### Battery test protocol

1. Charge the reference device to 100%, then unplug.
2. Set brightness to 50%, Wi-Fi on, mobile data off.
3. Start a scripted 60-minute session with the fake provider set to real-world latency (2 to 8 s per response) and a sandbox run every 3 minutes.
4. Turn the screen off.
5. After 60 minutes, record battery percent. Target: under 8% used.

## 9. Production builds

```
just release        # builds signed fullRelease APK and playRelease bundle
```

Outputs:
- `android/app/build/outputs/apk/full/release/` one universal APK (arm64-v8a and x86_64). Per-ABI split APKs are planned for P10
- `android/app/build/outputs/bundle/playRelease/` an Android App Bundle for Play

Release builds enable R8 and resource shrinking. JNA and UniFFI classes are kept by `core-bridge/consumer-rules.pro` and `app/proguard-rules.pro`. Without signing variables the release build is unsigned, so `just release` is only useful with them set.

## 10. Targets

| Setting | Value |
|---|---|
| `minSdk` | 29 (Android 10) |
| `compileSdk` | 37 |
| `targetSdk` | 36 now. Raise it to the level Google Play requires at release time; check before each release |
| ABIs | `arm64-v8a` (devices and Apple silicon emulators), `x86_64` (Intel emulators, CI, and a few devices) |
| Application IDs | `io.github.khanblair.tether` (play) and `io.github.khanblair.tether.full` (full), so both can be installed side by side |
| Kotlin packages | `app.tether.*`. The package name is separate from the application ID |

## 11. Signing

- `play`: upload key in the release keystore; Google Play App Signing holds the app signing key.
- `full`: signed with the release keystore directly. Users update from GitHub Releases; the signature never changes.
- Run `just release-signing` once, in your own terminal (it needs a terminal for the password prompt). It creates the keystore at `~/.tether-release/tether-release.keystore` (outside the repo) and stores it, with its password, as the four repository secrets `TETHER_KEYSTORE_BASE64`, `TETHER_KEYSTORE_PASSWORD`, `TETHER_KEY_ALIAS`, and `TETHER_KEY_PASSWORD`, plus the certificate fingerprint as the repository variable `TETHER_CERT_SHA256`. The password is typed at the prompt and never printed.
- The script never overwrites a keystore and refuses to replace secrets that already exist. If it stops after creating the keystore, run `just release-signing --existing` to store it. `--replace` allows replacing secrets, which locks `full` users out of updates once a release has shipped.
- `release.yml` pins the fingerprint: it fails when an APK is signed with any other key. It also fails with a clear message while any secret or the variable is missing.
- Back up the keystore and passwords in two separate offline places. Losing it means `full` users cannot update.
- Runtime pack and root filesystem manifests are signed with `TETHER_MANIFEST_KEY`. The matching public key is compiled into the app; rotating it needs an app release first.

## 12. Versions

- Semantic versioning: `MAJOR.MINOR.PATCH`.
- The version lives in one place: `version` under `[workspace.package]` in `core/Cargo.toml`. Gradle reads it for `versionName` and `versionCode`, and the core reports it through `version()`, so the app and the core cannot differ. The instrumented test `NativeLibraryTest` checks this on a device.
- `versionCode = MAJOR * 1_000_000 + MINOR * 10_000 + PATCH * 100 + ABI`, where ABI is 0 for the universal APK and bundles. The 1 and 2 values for `arm64-v8a` and `x86_64` apply once split APKs ship (P10).
- Database migrations are numbered and never edited after release.

## 13. Release steps

1. Confirm every P10 task for the release is done in progress-tracker.md.
2. Update `CHANGELOG.md` from merged PR titles; run `just slop CHANGELOG.md` on it.
3. Releases are automatic (D34): every merge to `main` that changes code publishes the next patch version. To publish a specific version instead (a minor or major, or a version you pick), run `just bump minor` (or `major`, or an exact `X.Y.Z`); it edits `core/Cargo.toml` and `core/Cargo.lock`, refuses a version that already has a tag, and merging that change publishes exactly that version. Gradle and the core both read the version.
4. Run `just check` and `just test-device` on the reference device.
5. Merge the pull request into `main`. There is no manual tag and no draft.
6. `release.yml` runs on every push to `main` except a push that only changes docs, Markdown, the prototype, or `.github/`. `tools/release-version.sh` picks the version: a version in `core/Cargo.toml` with no tag yet is published as it is; otherwise the next patch after the newest tag (a part stops at 99, then carries, because `versionCode` keeps two digits per part). The workflow sets that version in the build, builds both flavors, signs them with the keystore secrets, checks the signature against the pinned fingerprint, writes `SHA256SUMS`, and publishes the GitHub Release with the `full` APK and the `play` bundle, with notes from the merged pull requests. It then tries to push a `chore: release vX.Y.Z [skip release]` commit that sets the version in `core/Cargo.toml`; if that push is refused, the next run works from the tags. A failed release leaves no tag, so the next push tries again. Put `[skip release]` in a pull request title to skip one merge, or set the repository variable `AUTO_RELEASE` to `false` to stop automatic releases (the manual run in the Actions tab still works). Every release reaches all `full` users at once, with no staged rollout (D22).
7. Download the `full` APK from the release, install it on a clean device, and run the release smoke checklist (onboarding, one session per provider, sandbox run, CI run, Linux mode setup, rewind).
8. If the smoke test fails, fix it and merge: the next merge publishes the next patch. Delete the bad release only after that, and only if nobody installed it: `gh release delete vX.Y.Z --cleanup-tag`.
9. Upload the `play` bundle to the internal track, then closed testing, then production with staged rollout at 10%, 50%, 100% over a week.
10. Record the release in progress-tracker.md.

## 14. Troubleshooting

| Problem | Likely cause | Fix |
|---|---|---|
| `UnsatisfiedLinkError: libtether_core.so` | Core not built for the device's ABI | `just run` again; check the device ABI with `adb shell getprop ro.product.cpu.abi` |
| `cargo ndk` fails with "no such command" | cargo-ndk is not installed | `cargo install cargo-ndk --locked`, or `just setup` |
| Gradle fails "requires libraries and applications that depend on it to compile against version 37" | `compileSdk` is below what AndroidX needs | Install `platforms;android-37.0` with `sdkmanager` |
| Gradle wrapper download times out | Slow network to `services.gradle.org` | Run again; the wrapper timeout is 60 s with 3 retries |
| `NoClassDefFoundError` for JNA in release | R8 removed JNA classes | Check `proguard-rules.pro` keep rules |
| Bindings job fails in CI | Generated Kotlin out of date | `just bindings` and commit |
| `cargo ndk` cannot find the NDK | `ANDROID_NDK_HOME` unset or wrong version | Install the pinned NDK; `just setup` prints the expected path |
| Sandbox runs fail after an app update | Precompiled modules from an older Wasmtime | The cache key includes the engine hash; clear `files/runtimes/*/cwasm` from the Developer menu if it persists |
| Linux mode setup fails on the emulator | Emulator image lacks the needed syscalls or ptrace behavior | Test Linux mode on physical devices |
| Long runs stop on Samsung or Xiaomi | Battery optimization | Follow the in-app guidance; check "Never sleeping apps" on Samsung |
| Live smoke test returns 429 | Provider rate limit | Wait, or lower concurrency with `TETHER_SMOKE_SERIAL=1` |
| Approval notification does nothing when tapped | Notification permission denied on Android 13+ | Grant notifications in system settings |
