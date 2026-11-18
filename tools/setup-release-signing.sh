#!/usr/bin/env bash
# One-time setup of release signing. Creates the release keystore outside the repo, then stores it and
# its password as the four GitHub Actions secrets that release.yml needs.
#
# Run it in your own terminal, not through a chat. The password is typed at the prompt and is never
# printed, written into the repo, or passed on a command line.
#
# Usage: tools/setup-release-signing.sh
set -euo pipefail

dir="${TETHER_SIGNING_DIR:-$HOME/.tether-release}"
keystore="$dir/tether-release.keystore"
key_alias="tether"

command -v keytool >/dev/null 2>&1 || { echo "keytool not found. Install a JDK (17 or newer)."; exit 1; }
command -v gh >/dev/null 2>&1 || { echo "gh not found. Install the GitHub CLI and run 'gh auth login'."; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "gh is not signed in. Run 'gh auth login'."; exit 1; }

if [ -e "$keystore" ]; then
  echo "A keystore already exists at $keystore."
  echo "This script never overwrites one: losing the old key means full users cannot update."
  echo "To store the existing keystore as secrets again, move it aside first and restore it from your backup."
  exit 1
fi

repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
echo "Repository: $repo"
echo "Keystore will be created at: $keystore"
echo

read -r -s -p "Choose a keystore password (12 characters or more): " password
echo
read -r -s -p "Type it again: " again
echo
if [ "$password" != "$again" ]; then
  echo "The passwords differ."
  exit 1
fi
if [ "${#password}" -lt 12 ]; then
  echo "Use at least 12 characters."
  exit 1
fi

mkdir -p "$dir"
chmod 700 "$dir"

# keytool reads the password from the environment, so it never appears in the process list.
export TETHER_SIGNING_PASSWORD="$password"
trap 'unset password again TETHER_SIGNING_PASSWORD' EXIT

keytool -genkeypair -keystore "$keystore" -storetype PKCS12 -alias "$key_alias" \
  -keyalg RSA -keysize 4096 -validity 10000 -dname "CN=Tether, O=khanblair" \
  -storepass:env TETHER_SIGNING_PASSWORD -keypass:env TETHER_SIGNING_PASSWORD
chmod 600 "$keystore"

echo
echo "Certificate fingerprint (note it; it never changes for this key):"
keytool -list -v -keystore "$keystore" -alias "$key_alias" -storepass:env TETHER_SIGNING_PASSWORD | grep -E "SHA256:|Valid from"

echo
echo "Storing the four secrets in $repo ..."
base64 < "$keystore" | tr -d '\n' | gh secret set TETHER_KEYSTORE_BASE64 -R "$repo"
printf '%s' "$password" | gh secret set TETHER_KEYSTORE_PASSWORD -R "$repo"
printf '%s' "$key_alias" | gh secret set TETHER_KEY_ALIAS -R "$repo"
printf '%s' "$password" | gh secret set TETHER_KEY_PASSWORD -R "$repo"

echo
echo "Done. Next:"
echo "  1. Copy $keystore and the password into two separate offline places."
echo "     Losing either one means full users cannot update the app."
echo "  2. Do not commit the keystore. It lives outside the repo on purpose."
echo "  3. Merge the pull request that bumps the version in core/Cargo.toml. The release publishes itself."
