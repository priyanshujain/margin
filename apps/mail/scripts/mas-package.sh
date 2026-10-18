#!/usr/bin/env bash
set -euo pipefail

[ "$(uname -s)" = Darwin ] || { echo "mas-package: macOS is required." >&2; exit 1; }
app="${APP_PATH:-src-tauri/target/${MAS_TARGET:+$MAS_TARGET/}release/bundle/macos/Margin Mail.app}"
pkg="${PKG_PATH:-target-mas/Margin Mail.pkg}"
: "${APPLE_TEAM_ID:?set APPLE_TEAM_ID}"
: "${MAS_PROVISION_PROFILE:?set MAS_PROVISION_PROFILE}"
: "${MAS_AUTOFILL_PROVISION_PROFILE:?set MAS_AUTOFILL_PROVISION_PROFILE}"
: "${MAS_APP_IDENTITY:?set MAS_APP_IDENTITY to a Mac App Store application signing identity}"
: "${MAS_INSTALLER_IDENTITY:?set MAS_INSTALLER_IDENTITY to a Mac App Store installer signing identity}"
: "${MAS_BUILD_NUMBER:?set MAS_BUILD_NUMBER to a new App Store build number}"
[ -d "$app" ] || { echo "mas-package: build the app first: $app is missing." >&2; exit 1; }
extension="$app/Contents/PlugIns/MarginMailAutoFill.appex"
[ -d "$extension" ] || { echo "mas-package: the AutoFill extension is missing; use just package-appstore." >&2; exit 1; }
case "$MAS_APP_IDENTITY" in
  "3rd Party Mac Developer Application:"*|"Apple Distribution:"*) ;;
  *) echo "mas-package: a Mac App Store application identity is required." >&2; exit 1 ;;
esac
case "$MAS_INSTALLER_IDENTITY" in
  "3rd Party Mac Developer Installer:"*|"Mac Installer Distribution:"*) ;;
  *) echo "mas-package: a Mac App Store installer identity is required." >&2; exit 1 ;;
esac
case "$MAS_INSTALLER_IDENTITY" in
  *"($APPLE_TEAM_ID)") ;;
  *) echo "mas-package: the installer identity must belong to APPLE_TEAM_ID." >&2; exit 1 ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
security cms -D -i "$MAS_PROVISION_PROFILE" > "$work/host.profile"
security cms -D -i "$MAS_AUTOFILL_PROVISION_PROFILE" > "$work/extension.profile"
python3 - "$work" "$app" <<'PY'
import datetime
import os
import pathlib
import plistlib
import re
import sys

work, app = map(pathlib.Path, sys.argv[1:])
team = os.environ["APPLE_TEAM_ID"]
build = os.environ["MAS_BUILD_NUMBER"]
if not re.fullmatch(r"[A-Z0-9]{10}", team):
    sys.exit("mas-package: APPLE_TEAM_ID must contain 10 uppercase letters or digits.")
if not re.fullmatch(r"[1-9][0-9]{0,3}(?:\.[0-9]{1,2}){0,2}", build):
    sys.exit("mas-package: MAS_BUILD_NUMBER must be a valid numeric CFBundleVersion.")
for name, identifier, bundle in [
    ("host", "studio.margin.mail", app),
    ("extension", "studio.margin.mail.autofill", app / "Contents/PlugIns/MarginMailAutoFill.appex"),
]:
    profile = plistlib.loads((work / f"{name}.profile").read_bytes())
    authorized = profile["Entitlements"]
    prefix = profile["ApplicationIdentifierPrefix"][0]
    expected = f"{prefix}.{identifier}"
    if (profile["ExpirationDate"] <= datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
        or team not in profile.get("TeamIdentifier", [])
        or authorized.get("com.apple.application-identifier") != expected
        or authorized.get("com.apple.developer.team-identifier") != team
        or authorized.get("com.apple.developer.authentication-services.autofill-credential-provider") is not True
        or authorized.get("get-task-allow") is True
        or profile.get("ProvisionsAllDevices") or "ProvisionedDevices" in profile):
        sys.exit(f"mas-package: {name} profile is expired or is not the matching App Store distribution profile.")
    group = f"{team}.studio.margin.mail"
    if group not in authorized.get("com.apple.security.application-groups", []) and f"{team}.*" not in authorized.get("com.apple.security.application-groups", []):
        sys.exit(f"mas-package: {name} profile does not authorize the AutoFill app group.")
    template = pathlib.Path("src-tauri/entitlements.mas.plist" if name == "host" else "native/autofill/autofill.entitlements")
    entitlements = plistlib.loads(template.read_bytes())
    entitlements.update({
        "com.apple.application-identifier": expected,
        "com.apple.developer.team-identifier": team,
        "com.apple.developer.authentication-services.autofill-credential-provider": True,
        "com.apple.security.application-groups": [group],
    })
    (work / f"{name}.entitlements").write_bytes(plistlib.dumps(entitlements))
    info_path = bundle / "Contents/Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    if info["CFBundleIdentifier"] != identifier:
        sys.exit(f"mas-package: unexpected {name} bundle identifier.")
    info["CFBundleVersion"] = build
    info_path.write_bytes(plistlib.dumps(info))
PY

cp "$MAS_PROVISION_PROFILE" "$app/Contents/embedded.provisionprofile"
cp "$MAS_AUTOFILL_PROVISION_PROFILE" "$extension/Contents/embedded.provisionprofile"
find "$app" -type d -exec chmod go+rx {} +
find "$app" -type f -exec chmod go+r {} +
codesign --force --timestamp --options runtime --sign "$MAS_APP_IDENTITY" --entitlements "$work/extension.entitlements" "$extension"
codesign --force --timestamp --options runtime --sign "$MAS_APP_IDENTITY" --entitlements "$work/host.entitlements" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
codesign --display --extract-certificates="$work/signing-cert-" "$app"
python3 - "$work" <<'PY'
import pathlib
import plistlib
import sys
work = pathlib.Path(sys.argv[1])
certificate = (work / "signing-cert-0").read_bytes()
for name in ["host", "extension"]:
    profile = plistlib.loads((work / f"{name}.profile").read_bytes())
    if certificate not in profile.get("DeveloperCertificates", []):
        sys.exit(f"mas-package: signing certificate is not authorized by the {name} profile.")
PY
mkdir -p "$(dirname "$pkg")"
productbuild --component "$app" /Applications --sign "$MAS_INSTALLER_IDENTITY" "$pkg"
pkgutil --check-signature "$pkg"
echo "mas-package: wrote $pkg"
