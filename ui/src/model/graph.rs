//! The brick graph: bricks, tag rows and aggregated brick edges. No geometry.

use super::HotFn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrickView {
    pub name: String,
    /// First tag (the row), or `(untagged)`.
    pub tag: String,
    /// Row index into [`GraphView::rows`].
    pub row: usize,
    pub tags: Vec<String>,
    /// Configured members (crates or modules).
    pub members: Vec<String>,
    /// Highest value/limit ratio of any function with a limit (0 when none has one).
    #[serde(default)]
    pub worst_ratio: f64,
    /// Functions over a limit, worst first (see `score`).
    #[serde(default)]
    pub hot: Vec<HotFn>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeView {
    /// Brick indices.
    pub from: usize,
    pub to: usize,
    /// `uses` | `implements` | `reexports`.
    pub kind: String,
    /// Only test-scope references exist for this edge.
    pub test: bool,
    /// Ids of the `deps/*` findings of its module edges (empty when the edge is allowed).
    pub diags: Vec<String>,
}

/// Bricks, rows (tags in config order) and aggregated brick edges. No geometry: the layout runs
/// in [`crate::layout`], in the browser and on the server.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphView {
    pub rows: Vec<String>,
    pub bricks: Vec<BrickView>,
    pub edges: Vec<EdgeView>,
}
