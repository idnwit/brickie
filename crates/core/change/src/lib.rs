//! brickie-change: the versioned design. The `brickie.toml` tree is the design; every edit is
//! applied to the files at once, based on a version, and recorded in `.brickie/history/`.
//!
//! The op and version types are plain data in `brickie_model::design` (wasm-safe); this crate
//! is the engine (file system, `toml_edit`, the loader).
//!
//! Folders: `design` (versions, history, lock, writes, names to files), `edit` (documents and
//! ops), `preview` (virtual file set for the loader), `report` (diff, show, todo),
//! `scaffold` (code of new bricks).

mod design;
mod edit;
mod lock;
mod paths;
mod preview;
mod report;
mod scaffold;

#[cfg(test)]
mod tests;

pub use brickie_model::design::*;
pub use design::{CAP, Design, Files, Link, Outcome, Stale, ago, brick_path};
pub use preview::planned;
pub use report::{pending_text, planned_text};
