//! Client-side graph layout: a function-by-function port of the Python prototype viewer's
//! JavaScript (not published). Sugiyama style:
//! rows pinned by layer (here: tag order), dummy nodes for long edges, weighted crossing
//! minimisation (sweeps, transpose, sifting, restarts, annealing for <= 40 bricks), x by weighted
//! isotonic regression, ports on all four sides and smooth cubic routes.
//!
//! Pure and deterministic: [`layout`] has no randomness (fixed LCG seeds). Every function names
//! its JavaScript original. Deviations are listed in `docs/design.md`, step 3.

// The index loops and tuple-heavy tables mirror the original line by line.
#![allow(clippy::needless_range_loop, clippy::type_complexity)]

mod anneal;
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
pub const GAPY: f64 = 72.0;
pub const GAP: f64 = 10.0;
pub const DUM: f64 = 28.0;
pub const PADL: f64 = 40.0;
pub const PADR: f64 = 20.0;
pub const ITER: usize = 24;
pub const RESTARTS: usize = 30;
pub const REFK: usize = 12;
pub const LANE: f64 = 9.0;
/// `BZ.PORT` / `BZ.PAD` of the original.
pub const PORT: f64 = 12.0;
pub const PAD: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

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
    /// `BZ.N`: outward normal.
    pub fn normal(self) -> (f64, f64) {
        match self {
            Side::T => (0.0, -1.0),
            Side::B => (0.0, 1.0),
            Side::L => (-1.0, 0.0),
            Side::R => (1.0, 0.0),
        }
    }
    pub fn horizontal(self) -> bool {
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
    pub fn other(self) -> End {
        match self {
            End::A => End::B,
            End::B => End::A,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InBrick {
    pub id: String,
    /// Row (layer) index; rows are the tags in config order.
    pub row: usize,
}

#[derive(Debug, Clone)]
pub struct InEdge {
    /// Unique id; edges are processed in id order.
    pub id: String,
    pub a: usize,
    pub b: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Input {
    pub bricks: Vec<InBrick>,
    pub rows: Vec<String>,
    pub edges: Vec<InEdge>,
}

#[derive(Debug, Clone)]
pub struct RowOut {
    pub y: f64,
    pub h: f64,
    pub label: String,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Port {
    pub side: Side,
    pub off: f64,
}

/// Result of [`layout`] (the `L` object of the original).
#[derive(Debug, Clone)]
pub struct Layout {
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
    pub po: HashMap<(usize, End), Port>,
    pub routes: Vec<Option<Route>>,
    /// Weighted crossings of the final order (`bc`).
    pub cross: f64,
    /// Weighted crossings of the config order before minimisation (the baseline).
    pub cross0: f64,
    edges: Vec<(usize, usize)>,
}

/// `layoutFor` (one layout per graph, independent of layers, focus and filters).
pub fn layout(input: &Input) -> Layout {
    compute::compute_layout(input)
}

impl Layout {
    /// `routePath(e, L, P)`: the path of input edge `e` for the given box positions (dragged or
    /// not), or `None` for self edges.
    pub fn path(&self, e: usize, p: &[Rect]) -> Option<Route> {
        path::route_path(self, e, p)
    }
    pub fn edge_ends(&self, e: usize) -> (usize, usize) {
        self.edges[e]
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
