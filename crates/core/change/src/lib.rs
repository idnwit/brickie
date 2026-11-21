//! brickie-change: the versioned design. Design change sessions: op log, review, preview over
//! the loader, publish and scaffold.
//!
//! The published `brickie.toml` tree is what `check` reads. Every edit goes to the one draft
//! (`.brickie/draft/`), is based on a version, and is recorded in `.brickie/history/`;
//! [`Design::publish`] copies the draft into the real files once the code matches its
//! structure. This brick is the engine (file system, `toml_edit`, check's loader). The op and
//! version types are plain data in `brickie_model::design` (wasm-safe) and are re-exported
//! here. It does not decide whether an edge is allowed (that is `brickie-check`) and it does
//! not measure code.
//!
//! # Usage
//!
//! Every write names the head it is based on; a stale base is refused with [`Stale`]. An edit
//! is a list of [`Op`]s:
//!
//! ```
//! use brickie_change::{BrickSpec, By, Design, Op};
//!
//! # let dir = tempfile::tempdir().unwrap();
//! # let config = dir.path().join("brickie.toml");
//! # std::fs::write(&config, "[[brick]]\nname = \"core\"\nmembers = [\"demo-core\"]\n").unwrap();
//! let design = Design::at(&config).unwrap();
//! let head = design.head().unwrap();                  // the version the write is based on
//!
//! let op = Op::AddBrick {
//!     file: "brickie.toml".into(),
//!     brick: BrickSpec {
//!         name: "web".into(),
//!         members: vec!["demo-web".into()],
//!         ..BrickSpec::default()
//!     },
//! };
//! let out = design.apply(head.v, By::Agent, Some("add web".into()), vec![op]).unwrap();
//! assert!(out.changed);
//! assert_eq!(out.version.v, head.v + 1);              // a draft version; published files are untouched
//! assert!(design.config().unwrap().brick("web").is_some());   // the working tree is the draft
//! ```
//!
//! Review and publish:
//!
//! ```no_run
//! # use brickie_change::{Design, By};
//! # fn main() -> anyhow::Result<()> {
//! # let design = Design::at(std::path::Path::new("brickie.toml"))?;
//! let head = design.head()?;
//! let gate = design.publish_check(head.v)?;           // what still blocks: check on the draft
//! if gate.ok() {
//!     design.publish(head.v, By::Human, None)?;       // draft -> real files, one version
//! }                                                   // else `publish` fails with `Blocked`
//! design.undo(design.head()?.v, By::Human, None)?;    // revert the last version
//! # Ok(())
//! # }
//! ```
//!
//! # Key items
//!
//! - [`Design`]: one workspace's design. Reads: [`Design::head`], [`Design::log`],
//!   [`Design::diff`], [`Design::config`], [`Design::show_design`], [`Design::link_options`],
//!   [`Design::link_check`]. Writes: [`Design::apply`], [`Design::apply_scaffold`],
//!   [`Design::dry_run`], [`Design::publish`], [`Design::discard`],
//!   [`Design::undo`].
//! - [`Outcome`] and [`Plan`]: what a write did, and what it would do.
//! - [`Stale`] (exit code 3) and [`Blocked`] (exit code 4): the refusals of a write.
//! - [`working::load`] and [`Working`]: the design `check` reads (the draft, else the published
//!   files) with the code's findings.
//! - [`Op`], [`Version`], [`By`], [`Track`] and the other model types, re-exported.
//!
//! # Notes
//!
//! - Writes run under `.brickie/design.lock` and fail instead of waiting.
//! - [`Design::apply`] writes the draft only, never published files or source code;
//!   [`Design::apply_scaffold`] also creates the code of added bricks (never overwriting).
//! - A hand edit of the published files is recorded as an `external` version on the next read
//!   or write.
//! - Only the last [`CAP`] versions are kept.
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
