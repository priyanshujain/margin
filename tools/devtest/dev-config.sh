#!/bin/sh
# Usage: dev-config.sh BUNDLE_ID PRODUCT_NAME > dev.conf.json  (tauri --config overlay: dev identifier, renamed deep-link scheme, app bundle only, no pre-build hooks)
set -eu
[ $# -eq 2 ] || { echo "usage: dev-config.sh BUNDLE_ID PRODUCT_NAME" >&2; exit 2; }
case "$1" in *.dev) ;; *) echo "BUNDLE_ID must end in .dev, never a real app's id" >&2; exit 2;; esac
cat <<JSON
{
  "identifier": "$1",
  "productName": "$2",
  "build": { "beforeBuildCommand": "", "beforeBundleCommand": "" },
  "bundle": { "targets": ["app"], "createUpdaterArtifacts": false },
  "plugins": { "deep-link": { "desktop": { "schemes": ["$1"] }, "mobile": [] } }
}
JSON
