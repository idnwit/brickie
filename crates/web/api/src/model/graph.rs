//! The brick graph: bricks, tag rows and aggregated brick edges. No geometry.

use super::HotFn;
use serde::{Deserialize, Serialize};

/// The non-colour mark of an independent lib on its node and in the inspector.
pub const INDEPENDENT_LABEL: &str = "independent lib";

/// What an independent lib is, for tooltips.
pub const INDEPENDENT_TIP: &str = "independent lib: used like an external library. Edges into it are hidden unless they violate a rule; its own outgoing edges are errors unless allowed between independent libs.";

/// Shared layout constants for peer group container boxes (used by both layout input and SVG rendering).
pub const GROUP_HEADER_H: f64 = 48.0;
const GROUP_PAD: f64 = 10.0;
const GROUP_CARD_W: f64 = 128.0;
const GROUP_CARD_H: f64 = 42.0;
const GROUP_GAP: f64 = 8.0;
const GROUP_MAX_COLS: usize = 3;
const GROUP_EMPTY_W: f64 = 160.0;
const GROUP_EMPTY_H: f64 = 86.0;

/// Container `(width, height)` for a peer group with `n_members` member bricks.
pub fn group_box_size(n_members: usize) -> (f64, f64) {
    if n_members == 0 {
        return (GROUP_EMPTY_W, GROUP_EMPTY_H);
    }
    let cols = n_members.min(GROUP_MAX_COLS);
    let rows = n_members.div_ceil(GROUP_MAX_COLS);
    let w = 2.0 * GROUP_PAD + cols as f64 * GROUP_CARD_W + (cols - 1) as f64 * GROUP_GAP;
    let h = GROUP_HEADER_H
        + GROUP_PAD
        + rows as f64 * GROUP_CARD_H
        + (rows - 1) as f64 * GROUP_GAP
        + GROUP_PAD;
    (w, h)
}

/// Relative `(x, y, w, h)` of member card `index` inside a group container box.
pub fn group_member_rect(index: usize) -> (f64, f64, f64, f64) {
    let col = index % GROUP_MAX_COLS;
    let row = index / GROUP_MAX_COLS;
    let x = GROUP_PAD + col as f64 * (GROUP_CARD_W + GROUP_GAP);
    let y = GROUP_HEADER_H + GROUP_PAD + row as f64 * (GROUP_CARD_H + GROUP_GAP);
    (x, y, GROUP_CARD_W, GROUP_CARD_H)
}

/// One member brick displayed as a card inside a peer group container.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupMemberView {
    pub name: String,
    pub path: String,
    pub is_composite: bool,
    pub score: f64,
    pub worst_ratio: f64,
    pub errors: usize,
}

/// A forbidden dependency edge between two members of the same peer group.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InternalEdgeView {
    pub from_member: String,
    pub to_member: String,
    pub kind: String,
    pub test: bool,
    pub count: usize,
    pub diags: Vec<String>,
}

/// One brick of a level: its identity, tags, members, children and complexity heat. A faded
/// outside neighbour (`ghost`) is a brick of another level shown for context.
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
    /// An independent lib (`independent = true`): used like an external library. Inbound edges
    /// that are not violations are not drawn; see `hidden_in`.
    pub independent: bool,
    /// Bricks (as seen from this level) whose non-violating edges into this independent lib are
    /// hidden. A missing edge must not read as "no dependency", so the inspector says so.
    pub hidden_in: usize,
    /// Change preview: not a brick but the placeholder for modules of this composite that
    /// none of its bricks claims yet (`config/unassigned`); `members` covers them. Dropping
    /// it onto a brick adds those members there.
    pub unassigned: bool,
    /// True when this node represents a peer `[[group]]` container rather than a single brick.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_group: bool,
    /// Member bricks rendered as cards inside a peer group container.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub group_members: Vec<GroupMemberView>,
    /// Forbidden intra-group dependency edges between members of this peer group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_edges: Vec<InternalEdgeView>,
}

impl BrickView {
    /// Whether the brick has children (a composite) rather than being a leaf.
    pub fn composite(&self) -> bool {
        !self.children.is_empty()
    }
}

/// One aggregated dependency edge between two bricks of a level, with the findings and
/// evidence count behind it.
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
/// in the layout crate, in the browser and on the server.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphView {
    pub rows: Vec<String>,
    pub bricks: Vec<BrickView>,
    pub edges: Vec<EdgeView>,
}
