//! `brickie.toml`: the brick tree (one level per config file, composites via `path`), its
//! dependency and complexity rules, and the resolution of modules to leaf paths.

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

pub use decide::{Decision, decide};
pub use link::Link;
pub use load::{load, load_from, load_measure};
pub use measure::MeasureCfg;
pub use resolve::{Issue, Partition, Resolved, resolve};
pub use schema::{Action, Brick, ComplexityRule, Config, DepRule, KINDS, Scope, level_name};
pub use selector::Sel;
pub use source::{Disk, FileSource};
