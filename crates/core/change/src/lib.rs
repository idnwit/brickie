//! brickie-change: the versioned design. The published `brickie.toml` tree is what `check`
//! reads; every edit goes to the one draft (`.brickie/draft/`), based on a version, and is
//! recorded in `.brickie/history/`; `publish` copies the draft into the real files once the
//! code matches its structure.
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
pub use design::working::{self, Working};
pub use design::{Blocked, CAP, Design, Link, Outcome, Plan, Stale, ago, brick_path};
