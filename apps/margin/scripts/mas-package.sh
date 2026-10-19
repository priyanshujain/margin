#!/usr/bin/env bash
# Turn the .app that `tauri build` produced into a signed .pkg the App Store will accept, and
# optionally hand it to App Store Connect.
#
# Tauri has no App Store target, so everything after the bundle is done here: the provisioning
# profile goes in before signing (codesign hashes it), the entitlements carry the team identifier,
# and productbuild wraps the result. Tauri's own signing is deliberately not used, because it
# cannot embed a profile and would sign the app before the profile was in place.
set -euo pipefail

# CI builds universal; a local check against a single-arch build only needs to override this.
app="${APP_PATH:-src-tauri/target/universal-apple-darwin/release/bundle/macos/Margin.app}"
pkg="${PKG_PATH:-target-mas/Margin.pkg}"

: "${APPLE_TEAM_ID:?set APPLE_TEAM_ID to the 10-character team identifier}"
: "${MAS_PROVISION_PROFILE:?set MAS_PROVISION_PROFILE to the .provisionprofile path}"
: "${MAS_APP_IDENTITY:?set MAS_APP_IDENTITY, e.g. '3rd Party Mac Developer Application: Priyanshu Jain (TEAMID)'}"
: "${MAS_INSTALLER_IDENTITY:?set MAS_INSTALLER_IDENTITY, e.g. '3rd Party Mac Developer Installer: Priyanshu Jain (TEAMID)'}"

[ -d "$app" ] || { echo "mas-package: $app does not exist; run the build first." >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$(dirname "$pkg")"

# App Store Connect rejects an upload whose CFBundleVersion it has already seen, so a rejected
# build has to come back with a higher one. The marketing version stays put.
if [ -n "${MAS_BUILD_NUMBER:-}" ]; then
  /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $MAS_BUILD_NUMBER" "$app/Contents/Info.plist"
fi

cp "$MAS_PROVISION_PROFILE" "$app/Contents/embedded.provisionprofile"

# The profile is kept owner-only where it lives, because it sits next to signing keys, and cp
# carries that mode across. Apple rejects a package containing anything a non-root user cannot
# read, since the code signature could not then be verified at launch. Widen everything rather
# than just the profile, and only ever add permission bits, never remove one.
find "$app" -type d -exec chmod go+rx {} +
find "$app" -type f -exec chmod go+r {} +
sed "s/__TEAM_ID__/$APPLE_TEAM_ID/g" src-tauri/entitlements.mas.plist > "$work/entitlements.plist"
plutil -lint "$work/entitlements.plist" > /dev/null

# Nested code has to be signed before the bundle that contains it, and --deep is the wrong tool
# for signing (it applies the outer entitlements to everything inside). Tauri bundles carry no
# frameworks today, so this loop is usually empty, and it stays here so that stops being silent
# the day one appears.
while IFS= read -r -d '' nested; do
  codesign --force --timestamp --options runtime --sign "$MAS_APP_IDENTITY" "$nested"
done < <(find "$app/Contents/Frameworks" "$app/Contents/XPCServices" -maxdepth 1 -mindepth 1 -print0 2>/dev/null)

codesign --force --timestamp --options runtime \
  --sign "$MAS_APP_IDENTITY" \
  --entitlements "$work/entitlements.plist" \
  "$app"

codesign --verify --deep --strict --verbose=2 "$app"
echo "Entitlements on the signed bundle:"
codesign --display --entitlements - --xml "$app" | plutil -convert xml1 -o - -

productbuild --component "$app" /Applications --sign "$MAS_INSTALLER_IDENTITY" "$pkg"
pkgutil --check-signature "$pkg"
echo "mas-package: wrote $pkg"

# The upload needs the .p8 where altool looks for it; the caller places it and sets these.
# altool exits 0 even when it has just printed UPLOAD FAILED, so its exit status cannot be
# trusted and the transcript is the only reliable signal.
run_altool() {
  local action="$1" output
  output=$(xcrun altool "$action" -f "$pkg" -t macos \
    --apiKey "$APPLE_API_KEY_ID" --apiIssuer "$APPLE_API_ISSUER" 2>&1) || true
  printf '%s\n' "$output"
  if printf '%s' "$output" | grep -qE "VERIFY FAILED|UPLOAD FAILED|ERROR:"; then
    echo "mas-package: $action failed, see the errors above." >&2
    return 1
  fi
}

if [ -n "${MAS_UPLOAD:-}" ]; then
  : "${APPLE_API_KEY_ID:?}" "${APPLE_API_ISSUER:?}"
  run_altool --validate-app
  run_altool --upload-app
  echo "mas-package: uploaded to App Store Connect; the build appears once processing finishes."
fi
