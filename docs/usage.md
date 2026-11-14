# Usage

Install, commands, CI and building from source. For the quick start see [../README.md](../README.md).

## Install

Prebuilt binaries (Linux x86_64, macOS x86_64 and arm64), viewer bundle embedded:

```sh
curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh
# a specific build (default: latest versioned release, or `dev` if none yet)
curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh -s -- --version v0.1.0 --bin-dir ~/bin
```

- `--bin-dir` defaults to `~/.local/bin`. `BRICKIE_VERSION` and `BRICKIE_BIN_DIR` also work.
- `--dry-run` prints what would be downloaded.
- `dev` is a rolling prerelease rebuilt on every push to `main`.
- `brickie measure` needs `rust-analyzer` on `PATH` (`rustup component add rust-analyzer`).

From source (Rust 1.88+): `cargo build --release` gives `target/release/brickie`. The viewer bundle
is optional, see [Build the viewer bundle](#build-the-viewer-bundle). Not published on crates.io.

## Commands

```sh
brickie measure                  # slow: runs rust-analyzer, writes .brickie/cache.json
brickie check                    # fast: applies the rules to the cache, exits 1 on errors
brickie serve                    # viewer on http://127.0.0.1:7881
brickie interface root/web/api   # a brick's declared interface and its uses
brickie design show              # the design (brickie.toml tree) at its current version
brickie design rule add api store allow --reason "reads data" --base 15   # edit, versioned
brickie llms.txt                 # guide for coding agents
```

- Re-run `measure` after changing code. `check` and `serve` warn (`stale/*`) when the cache is older
  than the code. Changing only the rules never needs a re-measure.
- Every command takes `--config path/to/brickie.toml`.
- `measure --scip index.scip --workspace DIR`.
- `check --format text|json --color auto|always|never`.
- `serve --port 7881 --bind 127.0.0.1`.
- `design`: every write needs `--base N` (or `BRICKIE_BASE`), the version it was based on; a
  stale base writes nothing and exits 3. History is in `.brickie/history/` (git-ignore
  `.brickie/`). See `brickie design --help`.

## Accept a finding

A reviewer accepts a finding in `brickie.review.toml` next to the config, or with Accept in the viewer:

```toml
[[accept]]
id = "3fa9c1d2"
reason = "why this is fine"
```

## CI

`brickie check` exit codes:

| code | meaning |
| --- | --- |
| 0 | no unaccepted `error` findings (warnings and accepted findings do not fail) |
| 1 | at least one unaccepted `error` finding |
| 2 | configuration, review file or cache problem (for example no cache: run `measure`) |

A typical job runs `brickie measure && brickie check`. `--format json` prints `{summary,
diagnostics}`. Pin a released version for reproducible CI; `dev` and `latest` move.

```yaml
- uses: dtolnay/rust-toolchain@stable
  with: { components: rust-analyzer }
- run: curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh -s -- --version v0.1.0
- run: PATH="$HOME/.local/bin:$PATH" && brickie measure && brickie check
```

## Serve

`brickie serve` reads the cache and config on every request: re-run `measure` and refresh. The
header shows the cache age and how many files changed since. Its Measure button runs measure inside
the server (one at a time) and reloads when done. A measure holds `.brickie/cache.lock`; a second
one fails at once. Without the browser bundle the page is complete but read-only. With it you get
pan, zoom, dragging, filters and Accept. See [webui.md](webui.md).

## Build the viewer bundle

The wasm viewer is built from the `ui` crate and embedded at compile time, with the code viewer
(`/code/`). The script fetches a pinned Monaco release from the npm registry (needs curl, tar and
openssl once):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.123 --locked   # must match crates/web/ui/Cargo.toml
scripts/build-ui.sh        # writes assets/ui/ and assets/codeview/ (git-ignored)
cargo build --release      # embeds the bundles found there; without them it only warns
```
