# Everyday commands. See docs/development.md section 7.
# Run `just` with no arguments to list them.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Export variables from .env (signing keys, test keys) to every recipe.
set dotenv-load := true

# Application ID of the full flavor, used by `run`.
full_id := "io.github.khanblair.tether.full"
play_id := "io.github.khanblair.tether"

default:
    @just --list

# Check the toolchain and install what is missing.
setup:
    tools/setup.sh

# Build the core for the host.
core:
    cd core && cargo build --workspace

# Build fullDebug, install it on the connected device or emulator, and start it.
run:
    cd android && ./gradlew installFullDebug
    adb shell am start -n {{full_id}}/app.tether.MainActivity

# Same as run, for the play flavor.
run-play:
    cd android && ./gradlew installPlayDebug
    adb shell am start -n {{play_id}}/app.tether.MainActivity

# All Rust and Kotlin unit tests, plus the tooling tests.
test: test-core test-android test-tools

test-core:
    cd core && cargo test --workspace

test-android:
    cd android && ./gradlew testFullDebugUnitTest testPlayDebugUnitTest :core-bridge:testDebugUnitTest :designsystem:testDebugUnitTest

# Tests for slop-lint.
test-tools:
    cd tools/slop-lint && cargo test

# Instrumented tests on the connected device or emulator.
test-device:
    cd android && ./gradlew connectedFullDebugAndroidTest

# rustfmt, clippy, ktlint, detekt, Android lint, and the token contrast check.
lint:
    cd core && cargo fmt --all --check
    cd core && cargo clippy --workspace --all-targets -- -D warnings
    cd tools/slop-lint && cargo fmt --check && cargo clippy --all-targets -- -D warnings
    cd android && ./gradlew qualityCheck lintFullDebug lintPlayDebug
    node tokens/generate.ts --check

# Banned words and AI slop patterns in docs, README, and UI strings. Extra paths are checked too.
slop *extra:
    cargo run -q --manifest-path tools/slop-lint/Cargo.toml -- docs README.md android/app/src/main/res/values/strings.xml {{extra}}

# File length, plus a self test that proves the check fails on a long file.
smells:
    tools/check-file-length.sh --self-test
    tools/check-file-length.sh

# Prove Clippy and detekt fail on a function over the line limit.
smell-fixtures:
    tools/check-smell-fixtures.sh

# Fail if a commit on this branch touches more than 15 files.
commits:
    tools/check-commit-size.sh --self-test
    tools/check-commit-size.sh

# Regenerate the UniFFI Kotlin bindings.
bindings:
    tools/gen-bindings.sh

# Regenerate Tokens.kt and the Android color resources from tokens/tokens.json.
tokens:
    node tokens/generate.ts

# Render PNG brand assets from images/logo.svg.
icons:
    node tools/gen-icons.ts

# Build the docs site into site-dist/.
site:
    node tools/site/build.ts

# Serve the mobile UI prototype at http://127.0.0.1:4173/ (regenerates its token CSS first).
prototype:
    node tokens/generate.ts --css tools/prototype/tokens.css
    python3 -m http.server 4173 --bind 127.0.0.1 --directory tools/prototype

# Run before every push.
check: lint slop smells test
    tools/gen-bindings.sh
    git diff --exit-code -- android/core-bridge/src/main/kotlin/app/tether/core/generated

# Build the signed release artifacts. Needs the signing variables from docs/development.md section 5.
release:
    cd android && ./gradlew assembleFullRelease bundlePlayRelease
