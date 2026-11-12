#!/bin/sh
# Build the browser bundles: the viewer (wasm + JS glue) into assets/ui/, then the code viewer
# into assets/codeview/ (scripts/build-codeview.sh). The brickie server embeds these
# files when they exist at compile time (see crates/web/server/build.rs) and runs SSR-only otherwise.
#
# Prerequisites (one time):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.123 --locked   # must match crates/web/ui/Cargo.toml
#
# Usage: scripts/build-ui.sh [--dev]     (then `cargo build --release`)
set -eu
cd "$(dirname "$0")/.."
profile=release
flag=--release
if [ "${1:-}" = "--dev" ]; then profile=debug; flag=; fi
# Keep build-machine paths (home directory, toolchain location) out of the published bundle.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${HOME}=~ --remap-path-prefix=$(pwd)=."
cargo build -p brickie-ui --target wasm32-unknown-unknown --features hydrate $flag
mkdir -p assets/ui
wasm-bindgen --target web --no-typescript --out-dir assets/ui --out-name brickie_ui \
  "target/wasm32-unknown-unknown/$profile/brickie_ui.wasm"
ls -l assets/ui
# The code viewer bundle (embedded by codeview-host), same profile.
scripts/build-codeview.sh "$@"
