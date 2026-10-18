#!/bin/sh
# Usage: radicale.sh DIR [PORT] [USER] [PASSWORD]  (local CalDAV server via nix on 127.0.0.1 with one user and a "work" calendar; stop with: pkill -f "radicale --config DIR/config")
set -eu
[ $# -ge 1 ] || { echo "usage: radicale.sh DIR [PORT] [USER] [PASSWORD]" >&2; exit 2; }
dir=$1; port=${2:-5232}; user=${3:-tester}; pass=${4:-app-pass-123}
bin=$(nix build nixpkgs#radicale --no-link --print-out-paths)/bin/radicale
mkdir -p "$dir/data"
printf '%s:%s\n' "$user" "$pass" > "$dir/users"
cat > "$dir/config" <<CFG
[server]
hosts = 127.0.0.1:$port
[auth]
type = htpasswd
htpasswd_filename = $dir/users
htpasswd_encryption = plain
[storage]
filesystem_folder = $dir/data
[rights]
type = owner_only
CFG
nohup "$bin" --config "$dir/config" > "$dir/log" 2>&1 &
sleep 3
curl -s -u "$user:$pass" -X MKCALENDAR "http://127.0.0.1:$port/$user/work/" -o /dev/null -w "radicale on http://127.0.0.1:$port, calendar work: %{http_code}\n"
