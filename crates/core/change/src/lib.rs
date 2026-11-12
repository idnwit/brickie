//! brickie-change: change sessions over the `brickie.toml` tree. While a change is open,
//! nobody writes the config files: the human (viewer) and one coding agent (CLI) record ops
//! into `.brickie/change/`; the change is previewed (ops replayed onto the base snapshot in
//! memory), reviewed per op, and published (config files written, new bricks scaffolded).
//!
//! The op and record types are plain data in `brickie_model::change` (wasm-safe, shared with
//! the web api); this crate is the engine (file system, `toml_edit`, the loader).
//!
//! Folders: `store` (files, lock), `edit` (documents and ops), `session` (open, record,
//! review, replay, readers), `preview` (virtual file set for `check`), `report` (status, diff,
//! todo), `publish` (plan, write, scaffold).

mod edit;
mod paths;
mod preview;
mod publish;
mod report;
mod session;
mod store;

#[cfg(test)]
mod tests;

pub use brickie_model::change::*;
pub use preview::{Preview, planned};
pub use report::planned_text;
pub use session::{Change, Link, Log, Since};
