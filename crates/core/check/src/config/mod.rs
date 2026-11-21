//! The `brickie.toml` schema, loading, validation and module resolution.
//!
//! One `brickie.toml` (with its `include`s) is one level of the brick tree: bricks, `[[rule]]`
//! principles, `[[link]]` dependencies, `[[complexity]]` limits and, at the root only,
//! `[measure]`. A brick with `path` is a composite whose children live in `<path>/brickie.toml`.
//! This brick also owns the decision functions for dependencies ([`rule_allows`],
//! [`judge_link`], [`covering_link`]), shared by `check`, the publish gate, the design editor
//! and the viewer, so no rule is implemented twice. It reads config files only; it neither
//! reads the cache's functions and edges nor produces diagnostics.
//!
//! # Usage
//!
//! Load a tree, judge a link, and resolve measured modules to bricks:
//!
//! ```
//! use brickie_check::config::{self, judge_link};
//! use brickie_facts::{Crate, Module};
//!
//! let dir = tempfile::tempdir().unwrap();
//! let file = dir.path().join("brickie.toml");
//! std::fs::write(&file, r#"
//! [[brick]]
//! name = "core"
//! members = ["demo-core"]
//!
//! [[brick]]
//! name = "web"
//! members = ["demo-web"]
//!
//! [[rule]]
//! from = "web"
//! to = "core"
//! reason = "web renders what core computes"
//!
//! [[link]]
//! from = "web"
//! to = "core"
//! kinds = ["uses"]
//! reason = "web calls core's API"
//! "#).unwrap();
//!
//! let cfg = config::load(&file).unwrap();
//! assert_eq!(cfg.bricks.len(), 2);
//! // `None`: a rule allows the link (otherwise the reason it is denied).
//! assert_eq!(judge_link(&cfg, &cfg.links[0]), None);
//!
//! let krate = |name: &str| Crate {
//!     id: name.into(), package: name.into(), version: "0.1.0".into(), target: name.into(),
//!     kind: "lib".into(), root: format!("{name}/src/lib.rs"), dir: name.into(), publish: false,
//! };
//! let module = |name: &str| Module {
//!     id: name.into(), krate: name.into(), file: format!("{name}/src/lib.rs"),
//!     line: 1, doc: String::new(),
//! };
//! let (resolved, issues) = config::resolve(
//!     &cfg,
//!     &[krate("demo-core"), krate("demo-web")],
//!     &[module("demo-core"), module("demo-web")],
//!     dir.path(),
//! );
//! assert!(issues.is_empty());
//! assert_eq!(resolved.path("demo-web"), Some(&["web".to_string()][..]));
//! ```
//!
//! # Key items
//!
//! - [`load`], [`load_from`], [`load_measure`]: read the tree, from disk, from a
//!   [`FileSource`], or only `[measure]`.
//! - [`Config`] (one level), [`Brick`], [`Rule`], [`Link`], [`ComplexityRule`], [`Scope`],
//!   [`Sel`]: the schema.
//! - [`resolve`], [`Resolved`], [`Issue`]: modules to leaf paths, and the partition errors
//!   (`config/overlap`, `config/unassigned`, `config/member-outside-parent`).
//! - [`rule_allows`], [`judge_link`], [`covering_link`], [`link_cycles`], [`free`]: whether an
//!   edge is allowed.
//! - [`LinkState`]: a declared link with what the design says of it.
//!
//! # Notes
//!
//! - Unknown keys are rejected (`deny_unknown_fields`); `[measure]` is only valid in the root
//!   file; `include` and `path` cycles are errors.
//! - A brick whose `path` file does not exist stays a leaf and is recorded as `config/missing`
//!   in [`Config::missing`] instead of failing the load.
//! - `resolve` always returns a complete tree; partition errors are returned beside it.

mod decide;
mod link;
mod load;
mod measure;
mod member;
mod resolve;
mod schema;
mod selector;
mod source;
mod validate;

#[cfg(test)]
pub(crate) mod tests;

pub use decide::{Why, covering_link, free, judge_link, kinds_of, link_cycles, rule_allows};
pub use link::LinkState;
pub use load::{load, load_from, load_measure};
pub use measure::MeasureCfg;
pub use resolve::{Issue, Resolved, resolve};
pub use schema::{Brick, ComplexityRule, Config, KINDS, Link, Rule, Scope, level_name};
pub use selector::Sel;
pub use source::{Disk, FileSource};
