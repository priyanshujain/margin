#!/bin/sh
# Usage: launch-dev.sh APP_BUNDLE HOME_DIR  (opens the dev bundle with HOME and CFFIXED_USER_HOME at HOME_DIR; prints its PID)
set -eu
[ $# -eq 2 ] || { echo "usage: launch-dev.sh APP_BUNDLE HOME_DIR" >&2; exit 2; }
case "$2" in /tmp/*|/private/tmp/*) ;; *) echo "HOME_DIR must be under /tmp" >&2; exit 2;; esac
mkdir -p "$2"
open -n --env HOME="$2" --env CFFIXED_USER_HOME="$2" "$1"
sleep 5
bundle=$(cd "$1" && pwd -P)
pid=$(pgrep -f "$bundle/Contents/MacOS/" | tail -1)
[ -n "$pid" ] || { echo "nothing from $bundle is running" >&2; exit 1; }
echo "$pid"
