//! The port between the webview server and the viewer (UI): the endpoint contracts and every
//! type that crosses it. Wasm-safe (serde and `brickie-model` only), so both sides compile
//! against the same definitions and a contract change fails to build on either side.

mod endpoint;
mod model;
mod source;

pub use endpoint::*;
pub use model::*;
pub use source::{SourceLine, SourcePage};
