#!/bin/sh
# Build the code viewer bundle into assets/codeview/ (git-ignored): the page, the app (wasm + JS
# glue) and the Monaco files it needs. codeview-host embeds every file there at compile time
# (crates/lib/codeview/host/build.rs); without it `/code/` says how to build it.
#
# Prerequisites: as for scripts/build-ui.sh (wasm32 target, wasm-bindgen-cli 0.2.123), plus
# curl, tar and openssl to fetch Monaco once from the npm registry (pinned version, integrity checked).
#
# Usage: scripts/build-codeview.sh [--dev]     (then `cargo build`)
set -eu
cd "$(dirname "$0")/.."
MONACO=0.52.2
SHA512=GEQWEZmfkOGLdd3XK8ryrfWz3AIP8YymVXiPHEdewrUq7mh0qrKrfHLNCXcbB6sTnMLnOZ3ztSiKcciFUkIJwQ==
out=assets/codeview
profile=release
flag=--release
if [ "${1:-}" = "--dev" ]; then profile=debug; flag=; fi

# Monaco: only the editor core, the Rust grammar, the worker bootstrap and the icon font.
if [ "$(cat "$out/vs/VERSION" 2>/dev/null || true)" != "$MONACO" ]; then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  curl -sfL "https://registry.npmjs.org/monaco-editor/-/monaco-editor-$MONACO.tgz" -o "$tmp/m.tgz"
  got=$(openssl dgst -sha512 -binary "$tmp/m.tgz" | openssl base64 -A)
  if [ "$got" != "$SHA512" ]; then echo "monaco-editor $MONACO: integrity mismatch" >&2; exit 1; fi
  tar -xzf "$tmp/m.tgz" -C "$tmp" package/min/vs
  rm -rf "$out/vs"
  for f in loader.js editor/editor.main.js editor/editor.main.css base/worker/workerMain.js \
    basic-languages/rust/rust.js base/browser/ui/codicons/codicon/codicon.ttf; do
    mkdir -p "$out/vs/$(dirname "$f")"
    cp "$tmp/package/min/vs/$f" "$out/vs/$f"
  done
  echo "$MONACO" > "$out/vs/VERSION"
fi

# Keep build-machine paths (home directory, toolchain location) out of the published bundle.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${HOME}=~ --remap-path-prefix=$(pwd)=."
cargo build -p codeview-app --target wasm32-unknown-unknown $flag
wasm-bindgen --target web --no-typescript --out-dir "$out" --out-name codeview_app \
  "target/wasm32-unknown-unknown/$profile/codeview_app.wasm"
cp crates/lib/codeview/app/static/index.html crates/lib/codeview/app/static/glue.js \
  crates/lib/codeview/app/static/codeview.css "$out/"
du -sh "$out"
ls -l "$out/codeview_app_bg.wasm"
