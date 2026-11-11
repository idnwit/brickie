#!/bin/sh
# Install brickie from GitHub releases.
#   curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh
#   ... | sh -s -- --version dev --bin-dir ~/bin
# Options: --version dev|vX.Y.Z|latest (default latest; falls back to dev when no versioned
# release exists), --bin-dir DIR (default ~/.local/bin), --dry-run (print the plan, no network).
# Env: BRICKIE_VERSION, BRICKIE_BIN_DIR (flags win).
set -eu

REPO="idnwit/brickie"
BASE="https://github.com/${REPO}/releases"

version="${BRICKIE_VERSION:-latest}"
bin_dir="${BRICKIE_BIN_DIR:-${HOME:-}/.local/bin}"
dry_run=0

die() { echo "install.sh: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || die "--version needs a value"; version="$2"; shift 2 ;;
    --version=*) version="${1#*=}"; shift ;;
    --bin-dir) [ $# -ge 2 ] || die "--bin-dir needs a value"; bin_dir="$2"; shift 2 ;;
    --bin-dir=*) bin_dir="${1#*=}"; shift ;;
    --dry-run) dry_run=1; shift ;;
    -h|--help) sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

case "$version" in
  dev|latest|v[0-9]*) ;;
  [0-9]*) version="v${version}" ;;
  *) die "invalid --version '${version}' (use dev, latest or vX.Y.Z)" ;;
esac
[ -n "$bin_dir" ] || die "empty bin dir (set --bin-dir or HOME)"

os="${BRICKIE_TEST_OS:-$(uname -s)}"
arch="${BRICKIE_TEST_ARCH:-$(uname -m)}"
case "${os}/${arch}" in
  Linux/x86_64|Linux/amd64) target="x86_64-unknown-linux-gnu" ;;
  Darwin/x86_64) target="x86_64-apple-darwin" ;;
  Darwin/arm64|Darwin/aarch64) target="aarch64-apple-darwin" ;;
  *) die "unsupported platform ${os}/${arch} (supported: Linux x86_64, macOS x86_64 and arm64); build from source instead" ;;
esac

asset="brickie-${target}.tar.gz"

url_for() { # $1 = version (dev or vX.Y.Z)
  if [ "$1" = dev ]; then echo "${BASE}/download/dev/${asset}"
  else echo "${BASE}/download/$1/${asset}"; fi
}

fetch() { # $1 = url, $2 = output file
  if command -v curl >/dev/null 2>&1; then curl -fsSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then wget -q "$1" -O "$2"
  else die "need curl or wget"; fi
}

if [ "$version" = latest ]; then
  if [ "$dry_run" = 1 ]; then
    echo "version: latest (resolved at install time; falls back to dev if no versioned release)"
    url="${BASE}/latest/download/${asset}"
  else
    url="${BASE}/latest/download/${asset}"
    probe=$(mktemp)
    if ! fetch "${url}.sha256" "$probe" 2>/dev/null; then
      echo "install.sh: no versioned release yet, using the rolling dev build" >&2
      version=dev
      url=$(url_for dev)
    fi
    rm -f "$probe"
  fi
else
  url=$(url_for "$version")
fi

echo "version:  ${version}"
echo "target:   ${target}"
echo "url:      ${url}"
echo "sha256:   ${url}.sha256"
echo "bin dir:  ${bin_dir}"
[ "$dry_run" = 1 ] && exit 0

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
fetch "$url" "${tmp}/${asset}" || die "download failed: ${url}"
fetch "${url}.sha256" "${tmp}/${asset}.sha256" || die "download failed: ${url}.sha256"

expected=$(awk '{print $1}' "${tmp}/${asset}.sha256")
if command -v sha256sum >/dev/null 2>&1; then actual=$(sha256sum "${tmp}/${asset}" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then actual=$(shasum -a 256 "${tmp}/${asset}" | awk '{print $1}')
else die "need sha256sum or shasum to verify the download"; fi
[ -n "$expected" ] && [ "$expected" = "$actual" ] || die "checksum mismatch (expected ${expected}, got ${actual})"

tar -xzf "${tmp}/${asset}" -C "$tmp" brickie
mkdir -p "$bin_dir"
mv -f "${tmp}/brickie" "${bin_dir}/brickie"
chmod 755 "${bin_dir}/brickie"
echo "installed ${bin_dir}/brickie"
case ":${PATH}:" in
  *":${bin_dir}:"*) ;;
  *) echo "add it to your PATH:  export PATH=\"${bin_dir}:\$PATH\"" ;;
esac
echo "brickie measure also needs rust-analyzer on PATH (rustup component add rust-analyzer)."
