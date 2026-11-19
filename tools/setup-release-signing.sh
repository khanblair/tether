#!/usr/bin/env bash
# One-time setup of release signing. Creates the release keystore outside the repo, then stores it, its
# password, and the certificate fingerprint in the repository for release.yml.
#
# Run it in your own terminal, not through a chat. The password is typed at the prompt and is never
# printed, written into the repo, or passed on a command line.
#
# Usage: tools/setup-release-signing.sh              create a keystore and store it
#        tools/setup-release-signing.sh --existing   store the keystore that already exists on disk
#        tools/setup-release-signing.sh --replace    allow replacing secrets that are already set
#
# Replacing a key that shipped in a release means full users can no longer update the app.
set -euo pipefail

dir="${TETHER_SIGNING_DIR:-$HOME/.tether-release}"
keystore="$dir/tether-release.keystore"
key_alias="tether"
mode="new"
replace=0

for arg in "$@"; do
  case "$arg" in
    --existing) mode="existing" ;;
    --replace) replace=1 ;;
    -h | --help) sed -n '2,15p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg (try --help)"; exit 2 ;;
  esac
done

[ -t 0 ] || { echo "Run this in a terminal, so the password prompt works."; exit 1; }
keytool -help >/dev/null 2>&1 || { echo "keytool did not run. Install a JDK (17 or newer)."; exit 1; }
command -v gh >/dev/null 2>&1 || { echo "gh not found. Install the GitHub CLI and run 'gh auth login'."; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "gh is not signed in. Run 'gh auth login'."; exit 1; }

repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
existing_secrets="$(gh secret list -R "$repo" --json name --jq '.[].name' 2>/dev/null)" || {
  echo "Cannot read the secrets of $repo. Your gh login needs admin access to the repository."
  exit 1
}
if grep -qx TETHER_KEYSTORE_BASE64 <<<"$existing_secrets" && [ "$replace" -ne 1 ]; then
  echo "The release secrets already exist in $repo."
  echo "Replacing the signing key means full users can no longer update the app."
  echo "If you are sure, run this again with --replace."
  exit 1
fi

if [ "$mode" = "new" ] && [ -e "$keystore" ]; then
  echo "A keystore already exists at $keystore, and this script never overwrites one."
  echo "To store that keystore as the secrets, run: $0 --existing"
  echo "To create a new key instead, move the old keystore aside first (only before the first release)."
  exit 1
fi
if [ "$mode" = "existing" ] && [ ! -e "$keystore" ]; then
  echo "No keystore at $keystore. Run this without --existing to create one."
  exit 1
fi

echo "Repository: $repo"
echo "Keystore: $keystore"
echo

export TETHER_SIGNING_PASSWORD=""
trap 'unset password again TETHER_SIGNING_PASSWORD' EXIT

if [ "$mode" = "new" ]; then
  IFS= read -r -s -p "Choose a keystore password (12 characters or more): " password
  echo
  IFS= read -r -s -p "Type it again: " again
  echo
  if [ "$password" != "$again" ]; then
    echo "The passwords differ."
    exit 1
  fi
  if [ "${#password}" -lt 12 ]; then
    echo "Use at least 12 characters."
    exit 1
  fi
  TETHER_SIGNING_PASSWORD="$password"
  mkdir -p "$dir"
  chmod 700 "$dir"
  # keytool reads the password from the environment, so it never appears in the process list.
  keytool -genkeypair -keystore "$keystore" -storetype PKCS12 -alias "$key_alias" \
    -keyalg RSA -keysize 4096 -validity 10000 -dname "CN=Tether, O=khanblair" \
    -storepass:env TETHER_SIGNING_PASSWORD -keypass:env TETHER_SIGNING_PASSWORD
  chmod 600 "$keystore"
else
  IFS= read -r -s -p "Keystore password: " password
  echo
  TETHER_SIGNING_PASSWORD="$password"
  if ! keytool -list -keystore "$keystore" -alias "$key_alias" -storepass:env TETHER_SIGNING_PASSWORD >/dev/null 2>&1; then
    echo "That password does not open the keystore, or it has no '$key_alias' key."
    exit 1
  fi
fi

fingerprint="$(keytool -list -v -keystore "$keystore" -alias "$key_alias" -storepass:env TETHER_SIGNING_PASSWORD \
  | sed -n 's/^[[:space:]]*SHA256:[[:space:]]*//p' | head -n 1 | tr -d ':' | tr 'A-F' 'a-f')"
if ! grep -Eq '^[0-9a-f]{64}$' <<<"$fingerprint"; then
  echo "Could not read the certificate fingerprint from the keystore."
  exit 1
fi
echo
echo "Certificate SHA-256 (it never changes for this key): $fingerprint"

echo
echo "Storing the secrets and the fingerprint in $repo ..."
base64 < "$keystore" | tr -d '\n' | gh secret set TETHER_KEYSTORE_BASE64 -R "$repo"
printf '%s' "$password" | gh secret set TETHER_KEYSTORE_PASSWORD -R "$repo"
printf '%s' "$key_alias" | gh secret set TETHER_KEY_ALIAS -R "$repo"
printf '%s' "$password" | gh secret set TETHER_KEY_PASSWORD -R "$repo"
gh variable set TETHER_CERT_SHA256 --body "$fingerprint" -R "$repo"

echo
echo "Done. Next:"
echo "  1. Copy $keystore and the password into two separate offline places."
echo "     Losing either one means full users cannot update the app."
echo "  2. Do not commit the keystore. It lives outside the repo on purpose."
echo "  3. Merge the pull request that bumps the version in core/Cargo.toml. The release publishes itself."
echo "  The release fails if an APK is ever signed with a different key than the fingerprint above."
