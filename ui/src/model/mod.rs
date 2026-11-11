//! Data shared by the server (which builds it from the cache and diagnostics) and the browser
//! (which receives it as embedded JSON). Pure data and pure functions only.

mod diag;
mod function;
mod graph;
mod params;
mod score;

pub use diag::*;
pub use function::*;
pub use graph::*;
pub use params::*;
pub use score::*;

use serde::{Deserialize, Serialize};

/// Everything the page needs, embedded as JSON in the server-rendered HTML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageData {
    pub cache: CacheInfo,
    pub diags: Vec<DiagView>,
    pub graph: GraphView,
    pub functions: Vec<FnRow>,
}
