#!/bin/sh
# Usage: build-dev.sh APP_DIR CONFIG_JSON TARGET_DIR  (builds the frontend and a debug .app into TARGET_DIR; prints the bundle path)
set -eu
[ $# -eq 3 ] || { echo "usage: build-dev.sh APP_DIR CONFIG_JSON TARGET_DIR" >&2; exit 2; }
case "$3" in /tmp/*|/private/tmp/*) ;; *) echo "TARGET_DIR must be under /tmp, never the app's own target" >&2; exit 2;; esac
cd "$1"
mkdir -p "$3"
log="$3/build-dev.log"
./node_modules/.bin/vite build > "$log" 2>&1 || { tail -20 "$log" >&2; exit 1; }
CARGO_TARGET_DIR="$3" ./node_modules/.bin/tauri build --debug --bundles app --config "$2" >> "$log" 2>&1 || { tail -20 "$log" >&2; exit 1; }
ls -d "$3"/debug/bundle/macos/*.app
