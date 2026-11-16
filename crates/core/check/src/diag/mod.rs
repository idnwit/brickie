//! Diagnostics: the model and the text printer. JSON output is the serialisation
//! of [`Diagnostic`]; the text printer and the webview are consumers of the same value, so
//! nothing here is formatted for later parsing.

mod model;
mod short;
mod text;

pub use model::*;
pub use short::render_short;
pub use text::{Color, render_all};
