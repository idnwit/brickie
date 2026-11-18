//! brickie-check: load the brick config, evaluate the rules over the cache and render
//! diagnostics. [`check`] is the facade the CLI and the webview use.
//!
//! This crate reads `brickie.toml` (the brick tree, its rules, links and complexity limits) and
//! the measure cache, and turns what does not fit into [`diag::Diagnostic`] values. It never
//! measures code (that is `brickie-measure`) and never edits the design (that is
//! `brickie-change`).
//!
//! # Usage
//!
//! ```no_run
//! use brickie_check::{check, diag};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = Path::new("brickie.toml");
//! let loaded = check::load(config)?;                       // config + cache -> diagnostics
//! let root = check::config_dir(config);
//! print!("{}", diag::render_short(&loaded.diagnostics, &root));
//! let failed = !loaded.diagnostics.is_empty();             // the CLI exits non-zero then
//! # let _ = failed;
//! # Ok(())
//! # }
//! ```
//!
//! # Folder bricks
//!
//! Dependencies run `diag` and `config` <- `rules` / `interface` <- `check`.
//!
//! - [`diag`]: findings, diagnostics and their text rendering.
//! - [`config`]: the `brickie.toml` schema, loading, validation and module resolution, plus the
//!   one decision function for whether an edge is allowed ([`config::covering_link`]).
//! - `rules` (internal): evaluates dependency rules, cycles, complexity, unmapped and stale.
//! - [`interface`]: brick interfaces, their uses and `interface/unused`.
//! - [`check`]: the facade that combines them.
//!
//! # Notes
//!
//! Reads `brickie.toml` files and `.brickie/cache.json`; writes nothing. `check` never
//! re-measures: a changed file is reported as `stale/file` instead.

pub mod check;
pub mod config;
pub mod diag;
pub mod interface;
mod rules;
