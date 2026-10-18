#!/bin/sh
# Usage: ics-serve.sh DIR [PORT]  (serves DIR's .ics files on 127.0.0.1 for subscription tests; stop with: pkill -f "http.server PORT")
set -eu
[ $# -ge 1 ] || { echo "usage: ics-serve.sh DIR [PORT]" >&2; exit 2; }
port=${2:-8765}
cd "$1"
nohup python3 -m http.server "$port" --bind 127.0.0.1 > http.log 2>&1 &
sleep 1
echo "serving $(pwd) on http://127.0.0.1:$port/"
