#!/usr/bin/env bash
# Sign, package and upload an App Store build from this machine, using the certificates in
# ~/.margin-signing rather than the ones in CI.
#
# The certificates go into a keychain that exists only for the length of this run, and the login
# keychain is never written to. That keeps a machine that has signed once from quietly being able
# to sign forever, and it means this leaves nothing behind to go stale.
#
#   ./scripts/mas-upload-local.sh              # build, sign, package, upload
#   MAS_UPLOAD= ./scripts/mas-upload-local.sh  # stop after the .pkg
set -euo pipefail

cd "$(dirname "$0")/.."

DIR="${MARGIN_SIGNING_DIR:-$HOME/.margin-signing}"
BUNDLE_ID="${BUNDLE_ID:-studio.margin.app}"
KEYCHAIN="$(mktemp -d)/margin-mas.keychain-db"

for f in "$BUNDLE_ID.env" "$BUNDLE_ID.provisionprofile" apple-distribution.p12 mac-installer.p12; do
  [ -f "$DIR/$f" ] || { echo "mas-upload-local: $DIR/$f is missing; run apple-provision.rb first." >&2; exit 1; }
done

# shellcheck source=/dev/null
set -a; . "$DIR/$BUNDLE_ID.env"; set +a
export MAS_PROVISION_PROFILE="$DIR/$BUNDLE_ID.provisionprofile"

# App Store Connect refuses an upload whose build number it has already seen, and seconds since the
# epoch is both unique and monotonic without needing anything to be remembered between runs.
export MAS_BUILD_NUMBER="${MAS_BUILD_NUMBER:-$(date +%s)}"
export MAS_UPLOAD="${MAS_UPLOAD-1}"

if [ -n "${MAS_UPLOAD:-}" ]; then
  # shellcheck source=/dev/null
  set -a; . "$DIR/AuthKey.env"; set +a
  mkdir -p ~/private_keys
  cp "$DIR/AuthKey.p8" ~/private_keys/"AuthKey_$APPLE_API_KEY_ID.p8"
  chmod 600 ~/private_keys/"AuthKey_$APPLE_API_KEY_ID.p8"
fi

original_keychains=$(security list-keychains -d user | sed 's/^ *"//;s/"$//')
restore() {
  # shellcheck disable=SC2086
  security list-keychains -d user -s $original_keychains
  security delete-keychain "$KEYCHAIN" 2>/dev/null || true
}
trap restore EXIT

security create-keychain -p margin "$KEYCHAIN"
security unlock-keychain -p margin "$KEYCHAIN"
for cert in apple-distribution mac-installer; do
  security import "$DIR/$cert.p12" -k "$KEYCHAIN" -P "$(cat "$DIR/$cert.p12.pass")" \
    -T /usr/bin/codesign -T /usr/bin/productbuild > /dev/null
done
# Without this, codesign stops on a keychain prompt rather than signing.
security set-key-partition-list -S apple-tool:,apple: -k margin "$KEYCHAIN" > /dev/null
# shellcheck disable=SC2086
security list-keychains -d user -s "$KEYCHAIN" $original_keychains

if [ -z "${SKIP_BUILD:-}" ]; then
  pnpm tauri build --target universal-apple-darwin \
    --config src-tauri/tauri.appstore.conf.json --bundles app
fi

./scripts/mas-package.sh
