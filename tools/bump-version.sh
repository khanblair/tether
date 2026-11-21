#!/usr/bin/env bash
# Raises the core version in core/Cargo.toml and core/Cargo.lock. Merging the bump into main
# publishes the release: release.yml tags and publishes any version that has no tag yet.
#
# Usage: tools/bump-version.sh patch|minor|major   raise one part and reset the parts after it
#        tools/bump-version.sh X.Y.Z               set an exact version, which must be higher
#        tools/bump-version.sh --next X.Y.Z [kind]  print the next version, edit nothing
#        tools/bump-version.sh --self-test         prove the version math
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/core/Cargo.toml"

# The version in the workspace manifest, read the same way release.yml reads it.
current_version() {
  sed -n 's/^version = "\([0-9.]*\)"$/\1/p' "$manifest" | head -n 1
}

# The version after [current] for a bump kind (patch, minor, major) or an exact X.Y.Z.
# Minor and patch stop at 99 because versionCode packs each in two digits (see
# android/app/build.gradle.kts), so a bump past 99 carries into the next part. Fails for anything
# else, for an exact version with a minor or patch over 99, and for an exact version that is not
# higher than [current].
next_version() {
  local current="$1" kind="$2" major minor patch new_minor new_patch
  IFS=. read -r major minor patch <<<"$current"
  case "$kind" in
    patch)
      if [ "$patch" -ge 99 ]; then next_version "$current" minor; else echo "$major.$minor.$((patch + 1))"; fi
      ;;
    minor)
      if [ "$minor" -ge 99 ]; then next_version "$current" major; else echo "$major.$((minor + 1)).0"; fi
      ;;
    major) echo "$((major + 1)).0.0" ;;
    *)
      [[ "$kind" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || return 1
      IFS=. read -r _ new_minor new_patch <<<"$kind"
      [ "$new_minor" -le 99 ] && [ "$new_patch" -le 99 ] || return 1
      [ "$(printf '%s\n%s\n' "$current" "$kind" | sort -V | tail -n 1)" = "$kind" ] || return 1
      [ "$kind" != "$current" ] || return 1
      echo "$kind"
      ;;
  esac
}

self_test() {
  local failed=0
  expect() {
    local want="$1" got
    shift
    got="$(next_version "$@" 2>/dev/null || echo FAIL)"
    if [ "$got" != "$want" ]; then
      echo "next_version $*: wanted $want, got $got"
      failed=1
    fi
  }
  expect 0.0.4 0.0.3 patch
  expect 0.1.0 0.0.3 minor
  expect 1.0.0 0.9.12 major
  expect 0.9.13 0.9.12 patch
  expect 0.0.10 0.0.9 patch
  expect 0.2.0 0.0.3 0.2.0
  expect FAIL 0.0.3 0.0.3
  expect FAIL 0.0.3 0.0.2
  expect FAIL 0.0.3 0.0.10x
  expect FAIL 0.0.3 big
  expect 0.1.0 0.0.99 patch
  expect 1.0.0 0.99.99 patch
  expect 1.0.0 0.99.5 minor
  expect 0.0.99 0.0.98 patch
  expect FAIL 0.0.3 0.0.100
  expect FAIL 0.0.3 0.100.0
  [ "$failed" -eq 0 ] && echo "bump-version self test ok"
  return "$failed"
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit
fi
if [ "${1:-}" = "--next" ]; then
  next_version "${2:?usage: tools/bump-version.sh --next X.Y.Z [kind]}" "${3:-patch}"
  exit
fi

kind="${1:-}"
[ -n "$kind" ] || { echo "usage: tools/bump-version.sh patch|minor|major|X.Y.Z" >&2; exit 2; }
current="$(current_version)"
new="$(next_version "$current" "$kind")" || {
  echo "cannot move from $current with '$kind': use patch, minor, major, or a higher X.Y.Z" >&2
  exit 1
}
if git -C "$root" rev-parse -q --verify "refs/tags/v$new" >/dev/null; then
  echo "tag v$new already exists, so that version would never release" >&2
  exit 1
fi

tmp="$(mktemp)"
awk -v v="$new" '!done && /^version = "[0-9.]+"$/ { print "version = \"" v "\""; done = 1; next } { print }' \
  "$manifest" >"$tmp"
mv "$tmp" "$manifest"
# Only the workspace crates change version, so the offline index is enough on a developer machine.
# A fresh CI runner has no cached index and needs the network for the same command.
(cd "$root/core" && { cargo update --workspace --offline --quiet 2>/dev/null || cargo update --workspace --quiet; })
echo "version $current -> $new in core/Cargo.toml and core/Cargo.lock"
