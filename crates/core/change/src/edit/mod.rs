//! The config files of a change as `toml_edit` documents, and the ops applied to them in
//! place (comments and order kept).

mod apply;
mod brick;
mod docs;
mod element;
mod moves;
mod rules;
mod table;

pub use apply::apply;
pub use docs::Docs;
pub use element::{canonical, key};
pub use moves::target;
