//! Diagnostics: the model, the summary and the text printer. JSON output is the serialisation
//! of [`Diagnostic`]; the text printer and the webview are consumers of the same value, so
//! nothing here is formatted for later parsing.

mod model;
mod summary;
mod text;

pub use model::*;
pub use summary::summarize;
pub use text::{Color, render_all};
