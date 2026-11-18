#!/usr/bin/env bash
# Checks the toolchain and installs the pieces that can be installed without sudo.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ndk_version="$(sed -n 's/^tether.ndkVersion=//p' "$root/android/gradle.properties")"
failed=0

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing: $1 ($2)"
    failed=1
  fi
}

need rustup "https://rustup.rs"
need java "JDK 17 or newer; JDK 21 is what CI uses"
need node "Node 22.18 or newer"
need pnpm "https://pnpm.io/installation"
need just "https://just.systems"
need adb "Android SDK platform-tools"

if command -v rustup >/dev/null 2>&1; then
  (cd "$root" && rustup toolchain install)
  command -v cargo-ndk >/dev/null 2>&1 || cargo install cargo-ndk --locked --version "^4"
  command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked
fi

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -z "$sdk" ]; then
  echo "missing: ANDROID_HOME (path to the Android SDK)"
  failed=1
elif [ ! -d "$sdk/ndk/$ndk_version" ]; then
  echo "missing: NDK $ndk_version. Run: sdkmanager \"ndk;$ndk_version\""
  failed=1
fi

if command -v pnpm >/dev/null 2>&1; then
  (cd "$root" && pnpm install --frozen-lockfile)
fi

if [ "$failed" -ne 0 ]; then
  echo "setup: fix the items above, then run again"
  exit 1
fi
echo "setup: ok"
