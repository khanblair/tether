#!/usr/bin/env bash
# Fails when a commit touches more than 15 files. Merge commits are skipped.
#
# Usage: tools/check-commit-size.sh [base]       check commits in base..HEAD (default: origin/main)
#        tools/check-commit-size.sh --self-test  prove the check fails on a 16 file commit
set -euo pipefail

limit=15

check_range() {
  local range="$1" failed=0 commit count
  while IFS= read -r commit; do
    count="$(git show --name-only --format= "$commit" | grep -c . || true)"
    if [ "$count" -gt "$limit" ]; then
      echo "too many files: $(git log -1 --format='%h %s' "$commit") touches $count files, limit is $limit"
      failed=1
    fi
  done < <(git rev-list --no-merges "$range")
  return "$failed"
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  cd "$tmp"
  git init -q .
  git config user.email "selftest@example.com"
  git config user.name "Self test"
  git commit -q --allow-empty -m "base"
  base="$(git rev-parse HEAD)"
  seq 1 "$limit" | while read -r n; do echo "$n" > "ok-$n.txt"; done
  git add . && git commit -q -m "exactly the limit"
  check_range "$base..HEAD" >/dev/null
  seq 1 $((limit + 1)) | while read -r n; do echo "$n" > "big-$n.txt"; done
  git add . && git commit -q -m "over the limit"
  if check_range "$base..HEAD" >/dev/null; then
    echo "self test failed: a $((limit + 1)) file commit passed"
    exit 1
  fi
  echo "self test ok"
  exit 0
fi

check_range "${1:-origin/main}..HEAD"
echo "commit size ok"
