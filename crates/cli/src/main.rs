//! `brickie`: the command line. It measures Rust workspaces (module dependency edges, function
//! complexity) and checks them against the brick rules in `brickie.toml`.
//!
//! This binary only parses arguments, calls the crates that do the work and prints. Measuring is
//! `brickie-measure`, rules and diagnostics are `brickie-check`, the versioned design edits are
//! `brickie-change` and the webview is `brickie-web`. It holds no rule logic of its own, so the
//! CLI, the viewer and the design editor always agree. It is a binary crate: there is no library
//! API, and the examples below are shell command lines.
//!
//! # Usage
//!
//! The usual loop is measure, then check; `check --measure` does both when the cache is missing
//! or stale.
//!
//! ```text
//! brickie measure                         # write .brickie/cache.json (runs rust-analyzer)
//! brickie check --format short            # one line per finding, for coding agents
//! brickie check --measure --brick web/api # measure first if needed; only findings of one brick
//! brickie interface web/api               # the brick's declared items, docs and uses
//! brickie interface web/api --from web/ui # every site behind the edge web/ui -> web/api
//! brickie serve --port 7881               # reviewer webview at http://127.0.0.1:7881/
//! brickie llms.txt                        # the guide for coding agents
//! ```
//!
//! Edit the design (the `brickie.toml` tree) with versioned writes; each write needs `--base N`,
//! the version it is based on (read it with `design show`):
//!
//! ```text
//! brickie design show
//! brickie design brick add store --kind crate --base 15
//! brickie design rule add api store --reason "api may read data" --base 16
//! brickie design link add api store --kind uses --reason "reads orders" --base 17
//! brickie design undo --base 18
//! brickie design publish --base 18        # copy the draft into the real files
//! ```
//!
//! # Commands
//!
//! | Command | What it does |
//! |---------|--------------|
//! | `measure` | Reads the workspace and writes the cache; `--scip` reuses an index, `--workspace` sets the directory. |
//! | `check` | Prints findings from the cache; `--format text, json or short`, `--color`, `--brick` (repeatable), `--measure`. |
//! | `interface <brick>` | Prints a brick's interface; `--json`, `--from <brick>`. |
//! | `design ...` | Reads and edits the design: `show`, `log`, `diff`, `undo`, `apply`, `publish`, `discard`, `brick`, `rule`, `link`, `export`, `limit`, `include`. |
//! | `serve` | Starts the webview; `--port` (default 7881), `--bind` (default 127.0.0.1). |
//! | `llms.txt` | Prints the guide for coding agents. |
//!
//! Every command takes `--config <file>` (default `brickie.toml`) except `llms.txt`.
//!
//! # Exit codes and side effects
//!
//! - `check`: 0 clean, 1 findings, 2 config or cache problem. Planned work (todo) is printed but
//!   never counted.
//! - `interface`: 0, or 2 for a usage, config or cache problem.
//! - `design`: 0 ok; 2 any error (nothing written); 3 `design/stale` (the base is not the
//!   head); 4 `design/blocked` (`publish` while `check` fails). Writes go to a draft first and
//!   only `publish` changes the real files.
//! - `measure` writes `.brickie/cache.json` and takes a lock, so it never runs together with the
//!   serve job; hints and warnings go to stderr.
//! - `serve` edits files only through its guarded design and measure endpoints, and only when
//!   bound to loopback.

mod cli;

fn main() -> anyhow::Result<()> {
    let code = cli::run()?;
    std::process::exit(code);
}
