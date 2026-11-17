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

pub use decide::{Why, covering_link, free, judge_link, kinds_of, link_cycles, rule_allows};
pub use link::LinkState;
pub use load::{load, load_from, load_measure};
pub use measure::MeasureCfg;
pub use resolve::{Issue, Resolved, resolve};
pub use schema::{Brick, ComplexityRule, Config, KINDS, Link, Rule, Scope, level_name};
pub use selector::Sel;
pub use source::{Disk, FileSource};
