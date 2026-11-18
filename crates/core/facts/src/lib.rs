//! brickie-facts: the measure cache (`.brickie/cache.json`): facts only, no rules.
//!
//! The cache is what `brickie measure` writes and what `brickie check` and the viewer read: the
//! crates and modules of the workspace, and per source file its hash, its functions with
//! complexity numbers, its dependency edges and its public or crate-wide items. This brick
//! holds only the data model, hashing, the source-file walk and the write lock. It does not
//! produce facts (that is `brickie-measure`) and it does not judge them (that is
//! `brickie-check`).
//!
//! # Usage
//!
//! Readers deserialise a [`Cache`] and ask it for derived data; module-level edges are never
//! stored, [`Cache::edges`] aggregates them from the per-file edges:
//!
//! ```
//! use brickie_facts::{Cache, FileData, FileEdge};
//!
//! let mut cache = Cache::default();
//! cache.files.insert(
//!     "src/lib.rs".into(),
//!     FileData {
//!         sha256: brickie_facts::sha256_hex(b"fn main() {}"),
//!         compiled: true,
//!         size: 12,
//!         lines: 1,
//!         scope: "prod".into(),
//!         module: Some("demo".into()),
//!         functions: vec![],
//!         edges: vec![FileEdge {
//!             from: "demo".into(),
//!             to: "other".into(),
//!             kind: "uses".into(),
//!             scope: "prod".into(),
//!             line: 3,
//!             col: 5,
//!             end_col: 10,
//!             symbol: "other::run".into(),
//!             written: "other::run".into(),
//!         }],
//!         items: vec![],
//!     },
//! );
//! let edges = cache.edges();
//! assert_eq!(edges.len(), 1);
//! assert_eq!(edges[0].evidence[0].file, "src/lib.rs");
//!
//! // Doc helpers used by listings.
//! assert_eq!(brickie_facts::doc_summary("One line.\n\nMore."), "One line.");
//! ```
//!
//! A writer takes the lock for the whole run, so a second writer fails at once:
//!
//! ```no_run
//! use brickie_facts::CacheLock;
//! use std::path::Path;
//!
//! let _lock = CacheLock::acquire(Path::new(".brickie")).expect("another measure is running");
//! // ... write .brickie/cache.json; the lock file is removed when `_lock` drops.
//! ```
//!
//! # Key items
//!
//! - [`Cache`]: the whole file; [`Cache::edges`] derives module edges ([`Edge`], [`Evidence`]).
//! - [`FileData`], [`Function`], [`FileEdge`], [`Item`]: per-file facts, keyed by path.
//! - [`Crate`], [`Module`], [`Git`], [`Measured`]: workspace shape and how it was measured.
//! - [`sha256_hex`], [`walk_rs`], [`crate_dirs`]: hashing and finding source files.
//! - [`CacheLock`]: the single-writer lock, `.brickie/cache.lock`.
//! - [`SCHEMA_VERSION`]: bumped when the cache layout changes; a mismatch means re-measure.
//!
//! # Notes
//!
//! All file paths in the cache are relative to [`Cache::workspace_root`]. The lock file holds
//! the holder's PID; a lock whose process is gone is taken over, and the lock is removed on
//! drop.

mod files;
mod hash;
mod lock;
mod model;

pub use files::{crate_dirs, walk_rs};
pub use hash::sha256_hex;
pub use lock::CacheLock;
pub use model::*;
