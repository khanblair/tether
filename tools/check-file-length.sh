#!/usr/bin/env bash
# Fails when a source file is over the length limit in docs/code-standards.md.
# Limit: 400 lines, not counting tests. Rust inline tests start at `#[cfg(test)]` and are not counted.
# Generated files and test folders are skipped.
#
# Usage: tools/check-file-length.sh            check the repo
#        tools/check-file-length.sh --self-test  prove the check fails on a file over the limit
set -euo pipefail

limit=400
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

count_lines() {
  case "$1" in
    *.rs) awk '/^#\[cfg\(test\)\]/ { exit } { n++ } END { print n + 0 }' "$1" ;;
    *) wc -l < "$1" | tr -d ' ' ;;
  esac
}

check_files() {
  local failed=0 file lines
  while IFS= read -r file; do
    lines="$(count_lines "$file")"
    if [ "$lines" -gt "$limit" ]; then
      echo "too long: ${file#"$root"/} has $lines lines, limit is $limit"
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
