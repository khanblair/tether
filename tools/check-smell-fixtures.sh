#!/usr/bin/env bash
# Proves the smell gates fail where they should: a fixture function over the 50 line limit must
# fail Clippy (too_many_lines) and detekt (LongMethod).
#
# The fixtures are longer than the repo limit but shorter than each tool's own default (Clippy 100
# lines, detekt 60 lines), so they fail only while the repo's thresholds are in force.
#
# Usage: tools/check-smell-fixtures.sh [rust|kotlin|all]   (default: all)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# must_fail <name> <rule> <symbol> <dir> <command...>
# The command must exit non-zero, and its output must name both the rule and the fixture function.
must_fail() {
  local name="$1" rule="$2" symbol="$3" dir="$4"
  shift 4
  local out
  if out="$(cd "$dir" && "$@" 2>&1)"; then
    echo "$name fixture passed, but it must fail"
    return 1
  fi
  if ! grep -q "$rule" <<<"$out" || ! grep -q "$symbol" <<<"$out"; then
    echo "$name fixture failed, but not on $rule in $symbol. First lines of output:"
    head -n 15 <<<"$out"
    return 1
  fi
  echo "$name fixture fails on $rule in $symbol: ok"
}

# The Rust fixture sets its own lint and passes its own flags, so also check that the real setup
# still enables the lint and CI still denies warnings.
rust_config() {
  if ! grep -q 'too_many_lines' "$root/core/Cargo.toml"; then
    echo "core/Cargo.toml no longer enables clippy::too_many_lines"
    return 1
  fi
  if ! grep -q 'cargo clippy --workspace --all-targets -- -D warnings' "$root/.github/workflows/ci.yml"; then
    echo "ci.yml no longer runs clippy with -D warnings"
    return 1
  fi
}

rust() {
  rust_config
  must_fail rust too_many_lines long_function "$root/core/fixtures/smells" cargo clippy --quiet -- -D warnings
}

kotlin() {
  must_fail kotlin LongMethod longFunction "$root/android" ./gradlew -Ptether.smellFixtures=true :smell-fixtures:detekt --console=plain
}

case "${1:-all}" in
  rust) rust ;;
  kotlin) kotlin ;;
  all) rust && kotlin ;;
  *) echo "usage: $0 [rust|kotlin|all]"; exit 2 ;;
esac
