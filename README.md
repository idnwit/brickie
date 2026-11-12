# brickie

[![CI](https://github.com/idnwit/brickie/actions/workflows/ci.yml/badge.svg)](https://github.com/idnwit/brickie/actions/workflows/ci.yml)
[![dev release](https://github.com/idnwit/brickie/actions/workflows/dev-release.yml/badge.svg)](https://github.com/idnwit/brickie/actions/workflows/dev-release.yml)
[![release](https://github.com/idnwit/brickie/actions/workflows/release-binaries.yml/badge.svg)](https://github.com/idnwit/brickie/releases/latest)

brickie measures a Rust workspace and checks it against rules you write yourself.

- **Measure**: module-to-module dependency edges (`uses`, `implements`, `reexports`, with the
  exact source location and the path as written; the target is always the module that defines
  the item, even when reached through a re-export) from rust-analyzer's SCIP index, and per-function complexity
  (cognitive, cyclomatic, nesting, NPath-style paths, parameters, lines) from tree-sitter.
- **Check**: your `brickie.toml` groups crates and modules into *bricks* and says which bricks may
  depend on which, which cycles are forbidden, and how complex a function may be. brickie has no
  built-in architecture; the rules are yours.
- **View**: a local web page that shows bricks, edges, findings and the source next to each other,
  for reviewing what a coding agent changed.
- **Interfaces**: `brickie interface <brick>` (and the viewer) shows what a brick declares public,
  who uses each item, and which items nobody uses (`interface/unused`).
- **llms.txt**: `brickie llms.txt` prints a guide that tells coding agents how to fix each finding.

## Install

Prebuilt binaries (Linux x86_64, macOS x86_64 and arm64), with the viewer bundle embedded:

```sh
curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh
# a specific build; the default is the latest versioned release, or `dev` if there is none yet
curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh -s -- --version dev
curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh -s -- --version v0.1.0 --bin-dir ~/bin
```

`--bin-dir` defaults to `~/.local/bin`; `BRICKIE_VERSION` and `BRICKIE_BIN_DIR` work too, and
`--dry-run` prints what would be downloaded. `dev` is a rolling prerelease rebuilt on every push
to `main`. `brickie measure` needs `rust-analyzer` on `PATH` (`rustup component add rust-analyzer`).

From source (Rust 1.88 or newer). The interactive viewer is optional; see
[Building the viewer bundle](#building-the-viewer-bundle):

```sh
scripts/build-ui.sh            # optional: needs the wasm32 target and wasm-bindgen-cli
cargo build --release          # binary: target/release/brickie
```

brickie is not published on crates.io (the viewer bundle is generated, not part of the crate).

## Quick start

In the root of the workspace to analyse, create `brickie.toml` (see below), then:

```sh
brickie measure     # slow: runs rust-analyzer, writes .brickie/cache.json
brickie check       # fast: applies the rules to the cache, exits 1 on errors
brickie view        # opens the viewer on http://127.0.0.1:7881
brickie interface root/web/api   # a brick's declared interface and its uses
```

Re-run `brickie measure` after changing code; `check` and `view` warn (`stale/*`) when the cache
is older than the code. Changing only the rules never needs a re-measure.

Options: `--config path/to/brickie.toml` on every command, `brickie measure --scip index.scip
--workspace DIR`, `brickie check --format text|json --color auto|always|never`, `brickie view
--port 7881 --bind 127.0.0.1`.

## Configuration

```toml
include = ["presets/layered.toml"]   # optional: another file with the same schema

[[brick]]
name = "engine"
members = ["acme-engine"]            # package name, crate::module, or path:<dir>/
tags = ["core"]

[[brick]]
name = "adapters"
members = ["acme-backends"]
tags = ["adapter"]

[[brick]]
name = "web"
path = "crates/web"                  # composite: its children, rules and `exports` are in
tags = ["app"]                       # crates/web/brickie.toml (required)

[deps]
default = "allow"                    # for edges matching no rule: allow | deny
cycles = "deny"                      # cycles between bricks

[[deps.rule]]
from = "tag:core"                    # brick name | tag:<tag> | *
to = "tag:adapter"
kinds = ["uses", "implements"]       # default: all kinds
scope = "prod"                       # prod (default) | test | all
action = "deny"
reason = "core must not use adapters"

[[complexity]]
bricks = "tag:core"
cognitive = 15                       # a function fails above the limit
nesting = 3
paths = 200
params = 5
level = "error"                      # error (fails CI) | warn
```

The most specific matching rule wins (brick over tag over `*`; deny wins a tie). A composite
brick's file lists its children (which must cover all its modules), its own rules, and
`exports = ["api"]`, the children visible from outside (default none); an edge is checked by the
rules of the lowest level containing both ends. A full example
is in [docs/examples/brickie.toml](docs/examples/brickie.toml); the complete reference is in
[docs/llms.txt](docs/llms.txt).

A human reviewer can accept a finding in `brickie.review.toml` next to the config (or with the
Accept button in the viewer):

```toml
[[accept]]
id = "3fa9c1d2"
reason = "why this is fine"
```

## CI

`brickie check` exits with:

| code | meaning |
| --- | --- |
| 0 | no unaccepted `error` findings (warnings and accepted findings do not fail) |
| 1 | at least one unaccepted `error` finding |
| 2 | configuration, review file or cache problem (for example no cache: run `measure`) |

A typical job runs `brickie measure && brickie check`. `--format json` prints
`{summary, diagnostics}` for other tools.

GitHub Actions example:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    components: rust-analyzer
- name: Install brickie
  run: curl -fsSL https://raw.githubusercontent.com/idnwit/brickie/main/install.sh | sh -s -- --version v0.1.0
- name: Architecture check
  run: |
    export PATH="$HOME/.local/bin:$PATH"
    brickie measure && brickie check
```

Pin a released version for reproducible CI; `dev` and `latest` move.

## View

`brickie view` serves the viewer from the cache and the config; it re-reads both on every
request, so re-running `measure` and refreshing is enough. Without the browser bundle (below) the
page is complete but read-only; with it you get pan, zoom, dragging, filters and Accept.
Nested bricks are shown one level at a time: double-click a composite (marked with a magnifier icon) or click its icon to zoom in,
Backspace or the breadcrumb (top-left of the canvas) to zoom out; clicking a finding opens the level that reports it.

## For coding agents

`brickie llms.txt` prints a static guide (also in [docs/llms.txt](docs/llms.txt)): the workflow,
every diagnostic code with how to fix it, the config schema and the constraints agents must
respect (never edit the rules or the review file to make `check` pass).

## Building the viewer bundle

The interactive viewer is a wasm bundle built from the `ui` crate and embedded at compile time,
together with the code viewer (`/code/`: explorer, tabs, Monaco, findings in the code, F12 /
Shift+F12 navigation), whose script also fetches a pinned Monaco release from the npm registry
(needs curl, tar and openssl once):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.123 --locked   # must match crates/web/ui/Cargo.toml
scripts/build-ui.sh        # writes assets/ui/ and assets/codeview/ (git-ignored)
cargo build --release      # embeds the bundles found there
```

Without the bundle `cargo build` still works and prints a warning.

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace      # the measure test needs rust-analyzer on PATH
```

Design, cache format and roadmap: [docs/design.md](docs/design.md). Contributing and releases:
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
