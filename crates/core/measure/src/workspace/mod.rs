//! Cargo metadata, crates, git, rust-analyzer and the `Cargo.lock` guard.
//!
//! This brick gathers facts about the measured workspace and the external tools. It does not
//! read source files or the SCIP index (that is `scip`) and does not write the cache (that is
//! `run`).
//!
//! # Usage
//!
//! The brick is internal to the measure crate; `run` calls it in this order:
//!
//! ```text
//! let _guard = lock::LockGuard::new(&workspace);       // restores Cargo.lock on drop
//! let ws = metadata::load(&workspace)?;                 // packages and targets
//! let ids = crates::crate_ids(&ws);                     // lib: package name, else pkg/kind/target
//! let crates = crates::crates(&ws, &ids);               // brickie_facts::Crate rows
//! rust_analyzer::run(&ws.root, &out_scip, None)?;       // None: the host target
//! let git = git::git_info(&ws.root);                    // None outside a git repository
//! ```
//!
//! # Key items
//!
//! - [`metadata::load`] and [`metadata::Workspace`]: cargo packages and targets.
//! - [`crates::crate_ids`] and [`crates::crates`]: targets as cache crates.
//! - [`rust_analyzer::run`]: runs `rust-analyzer scip` with every cargo feature.
//! - [`git::git_info`]: commit and dirty flag.
//! - [`lock::LockGuard`]: puts `Cargo.lock` back when cargo or rust-analyzer changed it.
//!
//! # Notes
//!
//! Spawns `cargo metadata`, `rust-analyzer` and `git`. `rust_analyzer::run` runs the tool in its
//! own process group and kills what is left of it, so no rust-analyzer survives the call.

pub mod crates;
pub mod git;
pub mod lock;
pub mod metadata;
pub mod rust_analyzer;
