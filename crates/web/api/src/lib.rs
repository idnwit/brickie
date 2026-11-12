//! The port between the webview server and the viewer (UI): the endpoint contracts and every
//! type that crosses it. Wasm-safe (serde only), so both sides compile
//! against the same definitions and a contract change fails to build on either side.

mod endpoint;
mod model;

pub use endpoint::*;
pub use model::*;
