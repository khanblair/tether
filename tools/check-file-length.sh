#!/usr/bin/env bash
# Fails when a source file is over the length limit in docs/code-standards.md.
# Limit: 400 lines, not counting tests. Rust inline tests start at `#[cfg(test)]` and are not counted.
# A file that holds only tests (`*_tests.rs`, `*_test.rs`, `*Test.kt`, `*Tests.kt`) may have 800 lines:
# a test file is a list of independent cases, so splitting it adds files without making it easier to read.
# Generated files and test folders are skipped.
#
# Usage: tools/check-file-length.sh            check the repo
#        tools/check-file-length.sh --self-test  prove the check fails on a file over the limit
set -euo pipefail

limit=400
test_file_limit=800
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The line limit for one file: higher for a file that holds only tests.
limit_for() {
  case "$(basename "$1")" in
    *_tests.rs | *_test.rs | *Test.kt | *Tests.kt) echo "$test_file_limit" ;;
    *) echo "$limit" ;;
  esac
}

count_lines() {
  case "$1" in
    *.rs) awk '/^#\[cfg\(test\)\]/ { exit } { n++ } END { print n + 0 }' "$1" ;;
    *) wc -l < "$1" | tr -d ' ' ;;
  esac
}

check_files() {
  local failed=0 file lines max
  while IFS= read -r file; do
    lines="$(count_lines "$file")"
    max="$(limit_for "$file")"
    if [ "$lines" -gt "$max" ]; then
      echo "too long: ${file#"$root"/} has $lines lines, limit is $max"
      failed=1
    fi
  done
  return "$failed"
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  seq 1 $((limit + 1)) > "$tmp/Long.kt"
  seq 1 $((limit - 1)) > "$tmp/Short.kt"
  if printf '%s\n' "$tmp/Long.kt" | check_files >/dev/null; then
    echo "self test failed: a $((limit + 1)) line file passed"
    exit 1
  fi
  printf '%s\n' "$tmp/Short.kt" | check_files >/dev/null
  # A file that holds only tests may be longer, up to its own limit.
  seq 1 $((limit + 1)) > "$tmp/Big_tests.rs"
  seq 1 "$test_file_limit" > "$tmp/Edge_tests.rs"
  seq 1 "$test_file_limit" > "$tmp/EdgeTest.kt"
  seq 1 "$test_file_limit" > "$tmp/EdgeTests.kt"
  seq 1 "$test_file_limit" > "$tmp/Edge_test.rs"
  printf '%s\n' "$tmp/Big_tests.rs" "$tmp/Edge_tests.rs" "$tmp/Edge_test.rs" "$tmp/EdgeTest.kt" "$tmp/EdgeTests.kt" | check_files >/dev/null
  seq 1 $((test_file_limit + 1)) > "$tmp/TooBig_tests.rs"
  seq 1 $((test_file_limit + 1)) > "$tmp/TooBigTests.kt"
  for file in "$tmp/TooBig_tests.rs" "$tmp/TooBigTests.kt"; do
    if printf '%s\n' "$file" | check_files >/dev/null; then
      echo "self test failed: a $((test_file_limit + 1)) line test file passed"
      exit 1
    fi
  done
  # A name that only looks like a test file does not get the higher limit.
  for name in Latest.rs Contests.rs Attest.kt Protests.kt; do
    seq 1 $((limit + 1)) > "$tmp/$name"
    if printf '%s\n' "$tmp/$name" | check_files >/dev/null; then
      echo "self test failed: a $((limit + 1)) line file named $name passed"
      exit 1
    fi
  done
  echo "self test ok"
  exit 0
fi

cd "$root"
find core android tools -type f \( -name '*.rs' -o -name '*.kt' -o -name '*.kts' -o -name '*.ts' -o -name '*.sh' \) \
  -not -path '*/target/*' -not -path '*/build/*' -not -path '*/node_modules/*' \
  -not -path '*/generated/*' -not -path '*/tokens/Tokens.kt' \
  -not -path '*/tests/*' -not -path '*/test/*' -not -path '*/androidTest/*' -not -path '*/fixtures/*' \
  -print | sort | check_files
echo "file length ok"
