//! The brick graph: bricks, tag rows and aggregated brick edges. No geometry.

use super::HotFn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrickView {
    /// Name, local to the level (a faded neighbour shows its path below the common ancestor).
    pub name: String,
    /// Level path of the brick itself: `root/web/api` (a neighbour: the lifted outside brick).
    pub path: String,
    /// One-line description from the config (`desc`), if any.
    pub desc: Option<String>,
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
    /// Children names of a composite, in config order (empty for a leaf).
    pub children: Vec<String>,
    /// Exported children of a composite (drawn as ports on its border).
    pub exports: Vec<String>,
    /// Config file of a composite.
    pub file: Option<String>,
    /// Exported by the current level (a port of the frame).
    pub exported: bool,
    /// A faded neighbour outside the current level, with edges crossing its boundary.
    pub ghost: bool,
    /// Edit mode: in the design, but no measured module maps to it yet.
    pub planned: bool,
    /// Change preview: not a brick but the placeholder for modules of this composite that
    /// none of its bricks claims yet (`config/unassigned`); `members` covers them. Dropping
    /// it onto a brick adds those members there.
    pub unassigned: bool,
}

impl BrickView {
    pub fn composite(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    /// References (evidence sites) of the underlying module edges.
    pub count: usize,
    /// Worst level of its findings: `error` | `warn` (none when allowed).
    pub worst: Option<String>,
    /// A `deps/private` finding of this level from outside: drawn up to the frame wall.
    pub wall: bool,
    /// Client side only: when not empty, this edge aggregates these boundary edges for the
    /// "outside -> frame" mode; its inside end stands for the frame (see `model::boundary`).
    #[serde(skip)]
    pub under: Vec<usize>,
}

impl EdgeView {
    /// An aggregated edge between an outside neighbour and the frame.
    pub fn frame(&self) -> bool {
        !self.under.is_empty()
    }
}

/// Bricks, rows (tags in config order) and aggregated brick edges. No geometry: the layout runs
/// in [`crate::layout`], in the browser and on the server.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphView {
    pub rows: Vec<String>,
    pub bricks: Vec<BrickView>,
    pub edges: Vec<EdgeView>,
}
