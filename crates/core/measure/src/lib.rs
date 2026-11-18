//! brickie-measure: turns a Rust workspace into the measure cache (`.brickie/cache.json`).
//!
//! `brickie measure` asks cargo for the workspace's packages and targets, runs rust-analyzer to
//! get a SCIP index (symbols, definitions and references), measures function complexity with
//! tree-sitter, and writes everything as a [`brickie_facts::Cache`]. This brick only records
//! facts. It does not read `brickie.toml` bricks and does not judge anything: rules and
//! diagnostics are `brickie-check`.
//!
//! # Usage
//!
//! ```no_run
//! use brickie_measure::{run_with, Options};
//! use std::path::PathBuf;
//!
//! let opts = Options {
//!     config: PathBuf::from("brickie.toml"),
//!     scip: None,
//!     workspace: None,
//!     additional_targets: vec!["wasm32-unknown-unknown".into()],
//! };
//! // Runs cargo metadata and rust-analyzer; reports each step to the callback.
//! let cache_path = run_with(&opts, &|step| eprintln!("measure: {step}")).unwrap();
//! assert!(cache_path.ends_with(".brickie/cache.json"));
//! ```
//!
//! The decision tree of one function (the viewer draws it) is available without a run:
//!
//! ```
//! let src = "fn pick(x: u32) -> u32 { if x > 1 { 1 } else { 2 } }";
//! let tree = brickie_measure::fn_tree(src, 1).unwrap();
//! assert_eq!(tree.label, "pick");
//! ```
//!
//! # Folder bricks
//!
//! - `workspace`: cargo metadata, crates, git, rust-analyzer and the `Cargo.lock` guard.
//! - `scip`: pure normalisation of the SCIP index into file, symbol, module and occurrence rows.
//! - `run`: the flow that combines them and writes the cache; its public items are
//!   [`run_with`], [`Options`] and [`fn_tree`].
//!
//! # Notes
//!
//! - Side effects: spawns `cargo metadata` and `rust-analyzer scip`, writes
//!   `.brickie/cache.json` next to the config, and restores `Cargo.lock` if the tools changed it.
//! - The cache lock ([`brickie_facts::CacheLock`]) is held for the whole run; a concurrent
//!   measure of the same config fails at once.
//! - The cache is written to a temporary file and renamed, so readers never see a partial file.

#[cfg(test)]
mod e2e_tests;
mod run;
mod scip;
mod workspace;

pub use run::{Options, fn_tree, run_with};
