#!/bin/sh
# Usage: verify-unchanged.sh REAL_DATA_DIR BACKUP_DIR  (diff -r against the backup, then byte-compares every non-WAL file; WAL and SHM drift only while the owner's own app has the database open)
set -u
[ $# -eq 2 ] || { echo "usage: verify-unchanged.sh REAL_DATA_DIR BACKUP_DIR" >&2; exit 2; }
diff -r "$1" "$2" && { echo "identical to the backup"; exit 0; }
status=0
for f in "$2"/*; do
  name=$(basename "$f")
  case "$name" in *-wal|*-shm) continue;; esac
  if cmp -s "$f" "$1/$name"; then echo "same: $name"; else echo "CHANGED: $name"; status=1; fi
done
echo "processes holding the real files:"
lsof +D "$1" 2>/dev/null | awk 'NR > 1 { print $1, $2 }' | sort -u
exit $status
