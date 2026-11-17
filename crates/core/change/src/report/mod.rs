//! What the design reports: the semantic diff between versions, the tree as `design show`
//! prints it, the todo list and the per-file sheets.

pub mod diff;
pub use todo::design_todo;
mod sheets;
mod todo;
mod view;
