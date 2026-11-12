//! brickie-measure: `brickie measure`. cargo metadata and rust-analyzer SCIP (dependencies) plus
//! tree-sitter (complexity) -> `.brickie/cache.json`.
//!
//! Folder bricks (see `brickie.toml`): `workspace` (cargo and tool facts), `scip` (index rows),
//! `run` (the flow that combines them into the cache).

mod run;
mod scip;
mod workspace;

pub use run::{Options, fn_tree, run, run_with};
