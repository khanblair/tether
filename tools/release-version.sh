#!/usr/bin/env bash
# Decides the version the Release workflow publishes. Prints key=value lines for $GITHUB_OUTPUT:
# release, version, tag, and bump (true when the workflow must set the version in the build).
#
# - core/Cargo.toml holds a version with no tag yet: that version, chosen by hand (bump false).
# - Otherwise the next patch after the newest of Cargo.toml and the newest tag (bump true), so
#   every merge to main publishes a new, higher version without anyone editing a file.
#
# Usage: tools/release-version.sh              read core/Cargo.toml and the git tags
#        tools/release-version.sh --self-test  prove the decision on made-up tag lists
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Prints the four output lines for the Cargo.toml version [cargo] and a newline-separated
# list of existing tags [tags].
decide() {
  local cargo="$1" tags="$2" latest base next
  if ! printf '%s\n' "$tags" | grep -Fxq "v$cargo"; then
    printf 'release=true\nversion=%s\ntag=v%s\nbump=false\n' "$cargo" "$cargo"
    return
  fi
  latest="$(printf '%s\n' "$tags" | sed -n 's/^v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$/\1/p' | sort -V | tail -n 1)"
  base="$(printf '%s\n%s\n' "$cargo" "$latest" | sort -V | tail -n 1)"
  next="$("$root/tools/bump-version.sh" --next "$base" patch)"
  printf 'release=true\nversion=%s\ntag=v%s\nbump=true\n' "$next" "$next"
}

self_test() {
  local failed=0
  expect() {
    local want="$1" got
    shift
    got="$(decide "$@" | tr '\n' ' ')"
    if [ "$got" != "$want " ]; then
      echo "decide $*: wanted [$want], got [$got]"
      failed=1
    fi
  }
  # A hand-bumped version with no tag is published as it is.
  expect "release=true version=0.0.5 tag=v0.0.5 bump=false" 0.0.5 $'v0.0.3\nv0.0.4'
  expect "release=true version=0.0.1 tag=v0.0.1 bump=false" 0.0.1 ""
  # A tagged Cargo.toml version gets the next patch after the newest tag.
  expect "release=true version=0.0.5 tag=v0.0.5 bump=true" 0.0.4 $'v0.0.3\nv0.0.4'
  expect "release=true version=0.0.8 tag=v0.0.8 bump=true" 0.0.3 $'v0.0.3\nv0.0.7'
  # Ten is higher than nine: the order is by version, not by text.
  expect "release=true version=0.0.11 tag=v0.0.11 bump=true" 0.0.3 $'v0.0.3\nv0.0.9\nv0.0.10'
  # Tags that are not plain versions are ignored.
  expect "release=true version=0.0.5 tag=v0.0.5 bump=true" 0.0.4 $'v0.0.4\nv0.0.4-rc1\nnightly'
  # A patch at 99 carries into the minor (versionCode keeps two digits per part).
  expect "release=true version=0.1.0 tag=v0.1.0 bump=true" 0.0.99 $'v0.0.99'
  [ "$failed" -eq 0 ] && echo "release-version self test ok"
  return "$failed"
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit
fi

cargo="$(sed -n 's/^version = "\([0-9.]*\)"$/\1/p' "$root/core/Cargo.toml" | head -n 1)"
if ! printf '%s' "$cargo" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Cannot read a MAJOR.MINOR.PATCH version from core/Cargo.toml." >&2
  exit 1
fi
decide "$cargo" "$(git -C "$root" tag --list 'v*')"
