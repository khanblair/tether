#!/usr/bin/env bash
# Proves the smell gates fail where they should: a fixture function over the 50 line limit must
# fail Clippy (too_many_lines) and detekt (LongMethod).
#
# Usage: tools/check-smell-fixtures.sh [rust|kotlin|all]   (default: all)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

must_fail() {
  local name="$1" rule="$2" dir="$3"
  shift 3
  local out
  if out="$(cd "$dir" && "$@" 2>&1)"; then
    echo "$name fixture passed, but it must fail"
    return 1
  fi
  if ! grep -q "$rule" <<<"$out"; then
    echo "$name fixture failed, but not on $rule. First lines of output:"
    head -n 15 <<<"$out"
    return 1
  fi
  echo "$name fixture fails on $rule: ok"
}

rust() {
  must_fail rust too_many_lines "$root/core/fixtures/smells" cargo clippy --quiet -- -D warnings
}

kotlin() {
  must_fail kotlin LongMethod "$root/android" ./gradlew -Ptether.smellFixtures=true :smell-fixtures:detekt --console=plain
}

case "${1:-all}" in
  rust) rust ;;
  kotlin) kotlin ;;
  all) rust && kotlin ;;
  *) echo "usage: $0 [rust|kotlin|all]"; exit 2 ;;
esac
