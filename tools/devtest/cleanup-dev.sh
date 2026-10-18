#!/bin/sh
# Usage: cleanup-dev.sh BUNDLE_ID TARGET_DIR HOME_DIR  (quits the dev bundle, unregisters it, deletes its defaults, its /tmp target and its temp HOME)
set -u
[ $# -eq 3 ] || { echo "usage: cleanup-dev.sh BUNDLE_ID TARGET_DIR HOME_DIR" >&2; exit 2; }
case "$1" in *.dev) ;; *) echo "BUNDLE_ID must end in .dev, never a real app's id" >&2; exit 2;; esac
for d in "$2" "$3"; do
  case "$d" in /tmp/*|/private/tmp/*) ;; *) echo "$d is not under /tmp; refusing" >&2; exit 2;; esac
done
lsreg=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
for app in "$2"/debug/bundle/macos/*.app; do
  [ -d "$app" ] || continue
  pkill -f "$(cd "$app" && pwd -P)/Contents/MacOS/" 2>/dev/null
  "$lsreg" -u "$app" 2>/dev/null
done
defaults delete "$1" 2>/dev/null
rm -f "$HOME/Library/Preferences/$1.plist"
rm -rf "$2" "$3"
df -h "$HOME" | tail -1
