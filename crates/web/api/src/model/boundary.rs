//! How edges from outside neighbours are drawn inside a composite: to the children they reach
//! (`children`, the default) or to the frame (`frame`, URL `bnd=frame`). Frame mode uses edges
//! aggregated here from the level data; both sets live in one [`GraphView`] and the evaluation
//! shows one of them, so switching never re-lays out the level.

use super::{EdgeView, GraphView};
use std::collections::BTreeSet;

/// Name of the frame end in an edge key (`core>[frame]>uses`).
pub const FRAME_KEY: &str = "[frame]";

/// A boundary edge: one end is a faded outside neighbour, the other a child of the level.
pub fn is_boundary(g: &GraphView, e: &EdgeView) -> bool {
    !e.frame() && g.bricks[e.from].ghost != g.bricks[e.to].ghost
}

impl EdgeView {
    /// Merge boundary edge `e` (index `i` in the level graph) into this aggregate: `count` sums,
    /// `diags` union, `test` ands, `wall` ors.
    fn absorb(&mut self, e: &EdgeView, i: usize) {
        self.under.push(i);
        self.count += e.count;
        self.test &= e.test;
        self.wall |= e.wall;
        for d in &e.diags {
            if !self.diags.contains(d) {
                self.diags.push(d.clone());
            }
        }
    }
}

/// One aggregated edge per (outside neighbour, direction, kind) over the boundary edges of `g`,
/// in first-seen order. Its inside end is the first underlying child (an index only; it is drawn
/// at the frame). `count` sums, `diags` are the union, `test` holds when
/// every underlying edge is test-only, `wall` when any is.
pub fn frame_edges(g: &GraphView) -> Vec<EdgeView> {
    let mut out: Vec<EdgeView> = Vec::new();
    for (i, e) in g.edges.iter().enumerate() {
        if !is_boundary(g, e) {
            continue;
        }
        let into = g.bricks[e.from].ghost;
        let ghost = if into { e.from } else { e.to };
        let same = |x: &EdgeView| {
            x.kind == e.kind
                && g.bricks[x.from].ghost == into
                && (if into { x.from } else { x.to }) == ghost
        };
        match out.iter_mut().find(|x| same(x)) {
            Some(x) => x.absorb(e, i),
            None => out.push(EdgeView {
                under: vec![i],
                ..e.clone()
            }),
        }
    }
    out
}

/// The graph with the frame edges appended (indices of the original edges unchanged).
pub fn with_frame_edges(g: &GraphView) -> GraphView {
    let mut out = g.clone();
    out.edges.extend(frame_edges(g));
    out
}

/// The bricks an edge connects: both ends, and for a frame edge every child it stands for.
pub fn edge_bricks(g: &GraphView, e: usize) -> BTreeSet<usize> {
    let x = &g.edges[e];
    let mut s: BTreeSet<usize> = [x.from, x.to].into();
    for &u in &x.under {
        s.insert(g.edges[u].from);
        s.insert(g.edges[u].to);
    }
    s
}

/// The display name of an edge end (`[frame]` for the inside end of a frame edge).
pub fn end_name(g: &GraphView, e: usize, b: usize) -> &str {
    if g.edges[e].frame() && !g.bricks[b].ghost {
        FRAME_KEY
    } else {
        &g.bricks[b].name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BrickView;

    #[test]
    fn frame_mode_aggregates_per_neighbour_direction_and_kind() {
        let b = |name: &str, ghost| BrickView {
            name: name.into(),
            ghost,
            ..Default::default()
        };
        let e = |from, to, kind: &str, count, diags: &[&str], wall| EdgeView {
            from,
            to,
            kind: kind.into(),
            count,
            diags: diags.iter().map(|s| s.to_string()).collect(),
            wall,
            test: count == 1,
            ..Default::default()
        };
        let g = GraphView {
            rows: vec!["t".into()],
            // 0, 1 children; 2 outside source, 3 outside target.
            bricks: vec![
                b("a", false),
                b("b", false),
                b("app", true),
                b("core", true),
            ],
            edges: vec![
                e(2, 0, "uses", 3, &["d1"], false),
                e(2, 1, "uses", 2, &["d2", "d1"], true),
                e(2, 1, "implements", 1, &[], false),
                e(0, 3, "uses", 1, &[], false),
                e(1, 3, "uses", 1, &[], false),
                e(0, 1, "uses", 5, &[], false),
            ],
        };
        let f = frame_edges(&g);
        assert_eq!(f.len(), 3);
        let into = &f[0];
        assert_eq!((into.from, into.to, into.under.clone()), (2, 0, vec![0, 1]));
        assert_eq!(
            (into.count, into.diags.clone()),
            (5, vec!["d1".into(), "d2".into()])
        );
        assert!(into.wall && !into.test);
        assert_eq!(
            (f[1].kind.as_str(), f[1].under.clone()),
            ("implements", vec![2])
        );
        assert_eq!((f[2].from, f[2].to, f[2].under.clone()), (0, 3, vec![3, 4]));
        assert!(f[2].test);
        let g = with_frame_edges(&g);
        assert_eq!(g.edges.len(), 9);
        assert_eq!(edge_bricks(&g, 6), [0, 1, 2].into());
        assert_eq!(end_name(&g, 8, 0), FRAME_KEY);
        assert!(is_boundary(&g, &g.edges[0]) && !is_boundary(&g, &g.edges[5]));
        assert!(!is_boundary(&g, &g.edges[6]));
    }

    /// Two boundary edges from the same outside neighbour merge into one: `test` ands, `wall` ors.
    #[test]
    fn merging_ands_test_ors_wall() {
        let g = |tests: [bool; 2], walls: [bool; 2]| {
            let edges = (0..2)
                .map(|i| EdgeView {
                    from: 2,
                    to: i,
                    kind: "uses".into(),
                    count: 1,
                    test: tests[i],
                    wall: walls[i],
                    ..Default::default()
                })
                .collect();
            let b = |ghost| BrickView {
                ghost,
                ..Default::default()
            };
            GraphView {
                rows: vec![],
                bricks: vec![b(false), b(false), b(true)],
                edges,
            }
        };
        let f = frame_edges(&g([true, false], [false, true]));
        assert!(!f[0].test && f[0].wall && f[0].count == 2);
        let f = frame_edges(&g([true, true], [false, false]));
        assert!(f[0].test && !f[0].wall);
    }
}
