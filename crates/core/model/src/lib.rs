//! brickie's shared vocabulary: light, wasm-safe types used by both the engine (`brickie-core`)
//! and the viewer (`brickie-ui`). Pure data, no I/O.

pub mod codes;
pub mod design;
mod diagnostic;
mod level;
mod tree;

pub use diagnostic::{DiagEdge, Diagnostic, Note, RuleRef, Span};
pub use level::Level;
pub use tree::{FnTree, TreeNode};
