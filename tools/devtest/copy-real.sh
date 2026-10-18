#!/bin/sh
# Usage: copy-real.sh REAL_DATA_DIR DEST_DIR [--keep-secrets]  (cp -Rp a copy for compatibility checks; drops tokens.enc and tokens.salt unless asked, so nothing can reach a provider with real credentials)
set -eu
[ $# -ge 2 ] || { echo "usage: copy-real.sh REAL_DATA_DIR DEST_DIR [--keep-secrets]" >&2; exit 2; }
case "$2" in /tmp/*|/private/tmp/*) ;; *) echo "DEST_DIR must be under /tmp" >&2; exit 2;; esac
[ ! -e "$2" ] || { echo "$2 exists; pick a fresh path" >&2; exit 2; }
cp -Rp "$1" "$2"
if [ "${3:-}" != "--keep-secrets" ]; then rm -f "$2/tokens.enc" "$2/tokens.salt"; fi
ls -la "$2"
