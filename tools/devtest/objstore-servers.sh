#!/bin/sh
# Usage: objstore-servers.sh start|stop [DIR]  (MinIO on 127.0.0.1:9100 and WsgiDAV on 127.0.0.1:9200 for the objstore tests; WsgiDAV because rclone ignores If-Match)
set -eu
cmd=${1:-start}; dir=${2:-/tmp/objstore-devtest}
s3_port=9100; dav_port=9200
case $cmd in
stop)
  pkill -f "minio server $dir/minio" || true
  pkill -f "wsgidav.* --config $dir/wsgidav.yaml" || true
  exit 0 ;;
start) ;;
*) echo "usage: objstore-servers.sh start|stop [DIR]" >&2; exit 2 ;;
esac
minio=$(NIXPKGS_ALLOW_INSECURE=1 nix build --impure nixpkgs#minio --no-link --print-out-paths)/bin/minio
mc=$(nix build nixpkgs#minio-client --no-link --print-out-paths)/bin/mc
python=$(nix build --impure --no-link --print-out-paths --expr '(builtins.getFlake "nixpkgs").legacyPackages.${builtins.currentSystem}.python3.withPackages (p: [p.wsgidav p.cheroot])')/bin
mkdir -p "$dir/minio" "$dir/dav" "$dir/mc"
MINIO_ROOT_USER=objstore MINIO_ROOT_PASSWORD=objstore-secret nohup "$minio" server "$dir/minio" --address 127.0.0.1:$s3_port --console-address 127.0.0.1:$((s3_port + 1)) > "$dir/minio.log" 2>&1 &
cat > "$dir/wsgidav.yaml" <<CFG
host: 127.0.0.1
port: $dav_port
provider_mapping:
  "/": $dir/dav
simple_dc:
  user_mapping:
    "*":
      objstore:
        password: objstore-secret
http_authenticator:
  accept_basic: true
  accept_digest: false
  default_to_digest: false
CFG
nohup "$python/wsgidav" --config "$dir/wsgidav.yaml" > "$dir/dav.log" 2>&1 &
sleep 3
"$mc" --config-dir "$dir/mc" alias set local http://127.0.0.1:$s3_port objstore objstore-secret > /dev/null
"$mc" --config-dir "$dir/mc" mb --ignore-existing local/objstore-test > /dev/null
cat <<OUT
export OBJSTORE_S3_ENDPOINT=http://127.0.0.1:$s3_port OBJSTORE_S3_BUCKET=objstore-test OBJSTORE_S3_ACCESS_KEY=objstore OBJSTORE_S3_SECRET=objstore-secret
export OBJSTORE_WEBDAV_URL=http://127.0.0.1:$dav_port/ OBJSTORE_WEBDAV_USER=objstore OBJSTORE_WEBDAV_PASSWORD=objstore-secret
OUT
