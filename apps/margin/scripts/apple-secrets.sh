#!/usr/bin/env bash
# Push everything apple-provision.rb produced into this repo's GitHub Actions secrets.
#
# Values are piped from the files straight into `gh`, never echoed, so running this in a shared
# terminal or through an agent does not leak a signing key into the scrollback.
set -euo pipefail

DIR="${MARGIN_SIGNING_DIR:-$HOME/.margin-signing}"
BUNDLE_ID="${BUNDLE_ID:-studio.margin.app}"
REPO="${REPO:-priyanshujain/margin}"
ENV_FILE="$DIR/$BUNDLE_ID.env"

[ -f "$ENV_FILE" ] || { echo "apple-secrets: $ENV_FILE is missing; run apple-provision.rb first." >&2; exit 1; }
# shellcheck source=/dev/null
set -a; . "$ENV_FILE"; set +a

set_plain() {
  printf '%s' "$2" | gh secret set "$1" --repo "$REPO" --body -
  echo "  set $1"
}

set_b64() {
  [ -f "$2" ] || { echo "  skipped $1 ($2 is missing)"; return; }
  base64 -i "$2" | tr -d '\n' | gh secret set "$1" --repo "$REPO" --body -
  echo "  set $1"
}

set_file() {
  [ -f "$2" ] || { echo "  skipped $1 ($2 is missing)"; return; }
  gh secret set "$1" --repo "$REPO" < "$2"
  echo "  set $1"
}

echo "Signing identities and team:"
set_plain APPLE_TEAM_ID "$APPLE_TEAM_ID"
set_plain APPLE_SIGNING_IDENTITY "$APPLE_SIGNING_IDENTITY"
set_plain MAS_APP_IDENTITY "$MAS_APP_IDENTITY"
set_plain MAS_INSTALLER_IDENTITY "$MAS_INSTALLER_IDENTITY"

echo "Certificates:"
set_b64  APPLE_CERTIFICATE "$DIR/developer-id.p12"
set_file APPLE_CERTIFICATE_PASSWORD "$DIR/developer-id.p12.pass"
set_b64  MAS_APP_CERTIFICATE "$DIR/apple-distribution.p12"
set_file MAS_APP_CERTIFICATE_PASSWORD "$DIR/apple-distribution.p12.pass"
set_b64  MAS_INSTALLER_CERTIFICATE "$DIR/mac-installer.p12"
set_file MAS_INSTALLER_CERTIFICATE_PASSWORD "$DIR/mac-installer.p12.pass"
set_b64  MAS_PROVISION_PROFILE "$DIR/$BUNDLE_ID.provisionprofile"

echo "App Store Connect API key:"
if [ -f "$DIR/AuthKey.p8" ] && [ -f "$DIR/AuthKey.env" ]; then
  # shellcheck source=/dev/null
  set -a; . "$DIR/AuthKey.env"; set +a
  # An empty value here would be accepted by `gh` and then fail notarization as an auth error that
  # says nothing about a missing issuer, so refuse it at the point the mistake is still visible.
  if [ -z "${APPLE_API_KEY_ID:-}" ] || [ -z "${APPLE_API_ISSUER:-}" ]; then
    echo "  refused: AuthKey.env is missing APPLE_API_KEY_ID or APPLE_API_ISSUER." >&2
    exit 1
  fi
  set_plain APPLE_API_KEY_ID "$APPLE_API_KEY_ID"
  set_plain APPLE_API_ISSUER "$APPLE_API_ISSUER"
  set_b64   APPLE_API_KEY_P8 "$DIR/AuthKey.p8"
else
  echo "  skipped: put the .p8 at $DIR/AuthKey.p8 and write $DIR/AuthKey.env with"
  echo "           APPLE_API_KEY_ID= and APPLE_API_ISSUER=, then rerun."
fi

echo
echo "Not handled here: HOMEBREW_TAP_DEPLOY_KEY, an SSH deploy key on the tap rather than"
echo "anything Apple issued."
