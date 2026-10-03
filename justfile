set shell := ["bash", "-euo", "pipefail", "-c"]

app := "Margin"
bundle := "src-tauri/target/release/bundle/macos/" + app + ".app"

default:
    @just --list

dev:
    pnpm tauri dev

build:
    pnpm install --frozen-lockfile
    APPLE_SIGNING_IDENTITY=- pnpm tauri build --bundles app
    codesign --verify --deep --strict "{{bundle}}"

install: build
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -w /Applications ]; then
      dir=/Applications
    else
      dir="$HOME/Applications"
      mkdir -p "$dir"
    fi
    dest="$dir/{{app}}.app"
    if pgrep -x margin-app > /dev/null; then
      osascript -e 'tell application id "studio.margin.app" to quit'
      for _ in {1..40}; do
        pgrep -x margin-app > /dev/null || break
        sleep 0.25
      done
      if pgrep -x margin-app > /dev/null; then
        echo "Margin is still running. Quit it and run just install again." >&2
        exit 1
      fi
    fi
    rm -rf "$dest"
    ditto "{{bundle}}" "$dest"
    codesign --verify --deep --strict "$dest"
    echo "Installed {{app}} to $dest"
    open "$dest"
