//! `brickie.toml`: the brick tree (one level per config file, composites via `path`), its
//! dependency and complexity rules, and the resolution of modules to leaf paths.

mod load;
mod member;
mod resolve;
mod schema;
mod selector;
mod source;
mod validate;

#[cfg(test)]
pub(crate) mod tests;

pub use load::{load, load_from};
pub use resolve::{Resolved, resolve};
pub use schema::{
    Action, Brick, ComplexityRule, Config, DepRule, Export, KINDS, Level, level_name,
};
pub use selector::Sel;
pub use source::{Disk, FileSource};
