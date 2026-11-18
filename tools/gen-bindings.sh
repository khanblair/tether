#!/usr/bin/env bash
# Regenerates the UniFFI Kotlin bindings from the host build of the core.
# Output is committed; CI runs this script and fails on any diff.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$root/android/core-bridge/src/main/kotlin"

cd "$root/core"
cargo build -p tether-ffi

case "$(uname -s)" in
  Darwin) lib="target/debug/libtether_core.dylib" ;;
  *) lib="target/debug/libtether_core.so" ;;
esac

rm -rf "$out/app/tether/core/generated"
cargo run --quiet -p tether-ffi --features cli --bin uniffi-bindgen -- \
  generate --library "$lib" --language kotlin --no-format --out-dir "$out"

echo "Bindings written to $out/app/tether/core/generated"
