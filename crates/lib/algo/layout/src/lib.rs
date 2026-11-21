//! Layered graph layout and edge routing (Sugiyama style), pure Rust and wasm-safe.
//!
//! This brick turns a small directed graph whose nodes already sit in rows (layers) into box
//! positions and smooth edge routes. Rows are pinned by the caller (here: tag order); the brick
//! adds dummy nodes for edges that span several rows, minimises weighted crossings (sweeps,
//! transpose, sifting, restarts and annealing for up to 40 bricks), places boxes by weighted
//! isotonic regression, assigns ports on all four box sides and routes edges as cubic curves.
//! It does not draw anything, read files or know what a brick is: nodes are only ids with a row.
//! It is a function-by-function port of a JavaScript viewer, and every function names its original.
//!
//! # Usage
//!
//! ```
//! use brickie_layout::{layout, InBrick, InEdge, Input};
//!
//! let input = Input {
//!     rows: vec!["port".into(), "core".into()],
//!     bricks: vec![
//!         InBrick { id: "api".into(), row: 0 },
//!         InBrick { id: "engine".into(), row: 1 },
//!         InBrick { id: "store".into(), row: 1 },
//!     ],
//!     edges: vec![
//!         InEdge { id: "engine|api".into(), a: 1, b: 0 },
//!         InEdge { id: "store|api".into(), a: 2, b: 0 },
//!     ],
//! };
//! let l = layout(&input);
//! // One box per input brick, in input order.
//! assert_eq!(l.pos.len(), 3);
//! // One route per input edge; draw it as SVG path data.
//! for r in l.routes.iter().flatten() {
//!     let _svg_d: String = r.d();
//! }
//! // After dragging a box, re-route an edge for the new positions.
//! let moved = l.pos.clone();
//! let _again = l.path(0, &moved);
//! ```
//!
//! # Key items
//!
//! * [`layout`] computes a [`Layout`] from an [`Input`].
//! * [`InBrick`], [`InEdge`] and [`Input`] describe the graph; edges refer to bricks by index.
//! * [`Layout::path`] re-routes one edge for given (for example dragged) box positions.
//! * [`Rect`], [`Pt`], [`Side`], [`End`] and [`Port`] are the geometry of boxes and edge ends.
//! * [`Kind`] says how an edge travels (direct, arc over a row, or through dummy nodes) and
//!   [`Route`] holds its cubic segments.
//! * [`score_layout`] and [`measure_items`] count crossings, edge-through-box hits and direction
//!   changes, to compare layouts.
//!
//! # Notes
//!
//! * Pure and deterministic: there is no randomness (fixed LCG seeds), so equal input gives equal
//!   output.
//! * Edges are processed in `id` order, so ids must be unique. Self edges get no kind and no route.
//! * Deviations from the JavaScript original are listed in `docs/design/serve.md`, "Layout port".

// The index loops and tuple-heavy tables mirror the original line by line.
#![allow(clippy::needless_range_loop, clippy::type_complexity)]

mod anneal;
mod channels;
mod compute;
#[cfg(test)]
mod e2e_tests;
mod geometry;
mod measure;
mod order;
mod path;
mod place;
mod ports;
mod rank;
mod smart;

pub use geometry::Route;
pub use measure::{measure_items, score_layout};

use std::collections::HashMap;

/// `LAY` of the original.
pub const H: f64 = 54.0;
const GAPY: f64 = 96.0;
/// Row gap that no edge crosses (labels under the boxes only).
pub const GAPY_FREE: f64 = 42.0;
const GAP: f64 = 18.0;
const DUM: f64 = 28.0;
const PADL: f64 = 40.0;
const PADR: f64 = 20.0;
const ITER: usize = 24;
const RESTARTS: usize = 30;
pub const REFK: usize = 12;
const LANE: f64 = 9.0;
/// `BZ.PORT` / `BZ.PAD` of the original.
const PORT: f64 = 12.0;
const PAD: f64 = 10.0;

/// A point in layout coordinates (y grows downwards).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

/// An axis-aligned box: top-left corner `(x, y)` and size `w` by `h`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    /// Whether `q` lies strictly inside the box grown by margin `m` (negative `m` shrinks it).
    pub fn contains(&self, q: &Pt, m: f64) -> bool {
        q.x > self.x - m
            && q.x < self.x + self.w + m
            && q.y > self.y - m
            && q.y < self.y + self.h + m
    }

    /// Length of side `sd`: the width for top/bottom, the height for left/right.
    pub fn side_len(&self, sd: Side) -> f64 {
        if sd.horizontal() { self.h } else { self.w }
    }

    /// Extent as `[x0, x1, y0, y1]`, the layout of [`bbox`](geometry::bbox).
    pub(crate) fn bb(&self) -> [f64; 4] {
        [self.x, self.x + self.w, self.y, self.y + self.h]
    }
}

/// A box side (`'t' 'b' 'l' 'r'` in the original).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    T,
    B,
    L,
    R,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::T, Side::B, Side::L, Side::R];
    fn horizontal(self) -> bool {
        matches!(self, Side::L | Side::R)
    }
}

/// Which end of an edge: `a` = from, `b` = to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    A,
    B,
}

impl End {
    fn other(self) -> End {
        match self {
            End::A => End::B,
            End::B => End::A,
        }
    }
}

/// A node to lay out.
#[derive(Debug, Clone)]
pub struct InBrick {
    /// Display id; not used for ordering.
    pub id: String,
    /// Row (layer) index; rows are the tags in config order.
    pub row: usize,
}

/// A directed edge between two bricks, by index into [`Input::bricks`].
#[derive(Debug, Clone)]
pub struct InEdge {
    /// Unique id; edges are processed in id order.
    pub id: String,
    /// Index of the source brick.
    pub a: usize,
    /// Index of the target brick.
    pub b: usize,
}

/// The graph to lay out: bricks pinned to rows, and edges between them.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub bricks: Vec<InBrick>,
    /// Row labels, top to bottom; [`InBrick::row`] indexes this list.
    pub rows: Vec<String>,
    pub edges: Vec<InEdge>,
}

/// A laid-out row band.
#[derive(Debug, Clone)]
pub struct RowOut {
    /// Top of the band.
    pub y: f64,
    /// Height of the band.
    pub h: f64,
    pub label: String,
    /// Input brick indices in the row, left to right.
    pub ids: Vec<usize>,
}

/// How an edge travels (`L.routes[id]` of the original).
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Direct cubic between two sides.
    Bez,
    /// Same-row edge over (`top`) or under the row.
    Arc { top: bool, level: usize },
    /// Through dummy nodes; `chain` are node indices (bricks first, then dummies).
    Chain {
        chain: Vec<usize>,
        up_end: End,
        idx: usize,
        cnt: usize,
    },
}

/// Where an edge end attaches to a box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Port {
    pub side: Side,
    /// Offset along the side.
    pub off: f64,
}

/// Result of [`layout`] (the `L` object of the original).
#[derive(Debug, Clone)]
pub struct Layout {
    /// Total width and height of the drawing.
    pub w: f64,
    pub h: f64,
    pub px0: f64,
    /// Brick boxes by input index.
    pub pos: Vec<Rect>,
    /// Dummy nodes by node index (`None` for bricks): x centre, y top, height.
    pub dpos: Vec<Option<(f64, f64, f64)>>,
    pub rows: Vec<RowOut>,
    /// By input edge index; `None` for self edges.
    pub kinds: Vec<Option<Kind>>,
    /// Port of each edge end, by (input edge index, end).
    pub po: HashMap<(usize, End), Port>,
    /// Cubic route by input edge index; `None` for self edges.
    pub routes: Vec<Option<Route>>,
    /// Assigned vertical fraction per `(edge, hop)` in inter-row channels.
    pub(crate) tracks: HashMap<(usize, usize), f64>,
    /// Weighted crossings of the final order (`bc`).
    pub cross: f64,
    /// Weighted crossings of the config order before minimisation (the baseline).
    pub cross0: f64,
    edges: Vec<(usize, usize)>,
}

/// Lay out `input` (`layoutFor`; one layout per graph, independent of layers, focus and filters).
pub fn layout(input: &Input) -> Layout {
    compute::compute_layout(input)
}

impl Layout {
    /// `routePath(e, L, P)`: the path of input edge `e` for the given box positions (dragged or
    /// not), or `None` for self edges.
    pub fn path(&self, e: usize, p: &[Rect]) -> Option<Route> {
        path::route_path(self, e, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect {
        x: 10.0,
        y: 20.0,
        w: 100.0,
        h: 40.0,
    };

    #[test]
    fn contains_is_strict_and_grows_with_margin() {
        let at = |x, y| Pt { x, y };
        assert!(R.contains(&at(11.0, 21.0), 0.0));
        // on the border is not inside; a margin moves the border
        assert!(!R.contains(&at(10.0, 30.0), 0.0));
        assert!(R.contains(&at(9.0, 30.0), 2.0));
        assert!(!R.contains(&at(8.0, 30.0), 2.0));
        // a negative margin shrinks the box
        assert!(!R.contains(&at(12.0, 30.0), -2.0));
        assert!(R.contains(&at(12.5, 30.0), -2.0));
    }

    #[test]
    fn side_len_and_bb() {
        assert_eq!(R.side_len(Side::T), 100.0);
        assert_eq!(R.side_len(Side::B), 100.0);
        assert_eq!(R.side_len(Side::L), 40.0);
        assert_eq!(R.side_len(Side::R), 40.0);
        assert_eq!(R.bb(), [10.0, 110.0, 20.0, 60.0]);
    }
}
