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

# Tests for slop-lint and the version scripts.
test-tools:
    cd tools/slop-lint && cargo test
    tools/bump-version.sh --self-test
    tools/release-version.sh --self-test

# Build the full-flavor PRoot libraries and copy them into its arm64 source set.
proot:
    tools/proot/build.sh
    mkdir -p android/app/src/full/jniLibs/arm64-v8a
    cp build/proot/libproot.so android/app/src/full/jniLibs/arm64-v8a/libproot.so
    cp build/proot/libproot-loader.so android/app/src/full/jniLibs/arm64-v8a/libproot-loader.so

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
check: lint slop smells smell-fixtures test
    tools/gen-bindings.sh
    git diff --exit-code -- android/core-bridge/src/main/kotlin/app/tether/core/generated

# One-time: create the release keystore and store it, with its password, as GitHub secrets. Run in your own terminal.
release-signing:
    tools/setup-release-signing.sh

# Create the Linux rootfs signing key outside the repository.
linux-key:
    mkdir -p "$HOME/.tether-release"
    cargo run --quiet --manifest-path core/Cargo.toml --locked -p tether-linux-manifest -- keygen --out "$HOME/.tether-release/linux-manifest.key"
    echo "Next: run just linux-manifest after checking the pinned Alpine release."

# Download, verify, sign, and stage the Linux rootfs release files.
linux-manifest:
    #!/usr/bin/env bash
    set -euo pipefail
    source third_party/linux/pins.sh
    dist="dist/linux"
    mkdir -p "$dist"
    archive_name="alpine-minirootfs-$ALPINE_VERSION-$ALPINE_ARCH.tar.gz"
    alpine_url="https://dl-cdn.alpinelinux.org/alpine/$ALPINE_BRANCH/releases/$ALPINE_ARCH/$archive_name"
    archive="$dist/$archive_name"
    checksum="$archive.sha256"
    curl --fail --location --silent --show-error --output "$archive" "$alpine_url"
    curl --fail --location --silent --show-error --output "$checksum" "$alpine_url.sha256"
    expected="$(awk '{ for (i = 1; i <= NF; i++) if (length($i) == 64 && $i ~ /^[[:xdigit:]]+$/) { print tolower($i); exit }}' "$checksum")"
    actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
    test "$expected" = "$actual"
    release_asset_url="https://github.com/khanblair/tether/releases/download/rootfs-$ALPINE_VERSION/$archive_name"
    sign_output="$(cargo run --quiet --manifest-path core/Cargo.toml --locked -p tether-linux-manifest -- sign --key "$HOME/.tether-release/linux-manifest.key" --rootfs "$archive" --url "$release_asset_url" --alpine-version "$ALPINE_VERSION" --out-dir "$dist")"
    printf '%s\n' "$sign_output"
    public_key="$(printf '%s\n' "$sign_output" | sed -n 's/^public key: //p')"
    test -n "$public_key"
    manifest_url="https://github.com/khanblair/tether/releases/download/rootfs-$ALPINE_VERSION/manifest.json"
    echo "Next steps:"
    echo "1. Create the GitHub release tagged rootfs-$ALPINE_VERSION."
    echo "2. Upload these files to that release:"
    echo "   $archive"
    echo "   $checksum"
    echo "   $dist/manifest.json"
    echo "   $dist/manifest.json.sig"
    echo "3. Set manifest_url to $manifest_url."
    echo "4. Put this public key in android/app/src/full/res/values/linux.xml:"
    echo "   $public_key"

# Raise the core version: `just bump patch` (or minor, major, or an exact X.Y.Z). Merging the bump publishes the release.
bump level="patch":
    tools/bump-version.sh {{level}}

# Build the signed release artifacts. Needs the signing variables from docs/development.md section 5.
release:
    cd android && ./gradlew assembleFullRelease bundlePlayRelease
