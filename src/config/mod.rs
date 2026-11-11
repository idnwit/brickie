//! `brickie.toml`: bricks, dependency rules and complexity rules, with `include` merging.

mod load;
mod resolve;
mod schema;
mod selector;

pub use load::load;
pub use resolve::{hash, resolve};
pub use schema::{Action, Brick, ComplexityRule, Config, DepRule, KINDS, Level};
pub use selector::Sel;
