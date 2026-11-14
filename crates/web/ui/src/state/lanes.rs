//! The layout input of a level: its bricks in their tag lanes and its edges, plus
//! one edge per (from, to) brick pair of its planned rules, so the automatic layout orders the
//! lanes and bricks for the rules too and routes them around the boxes. The untagged lane carries
//! no tag semantics: it moves to the lane position that keeps the edges shortest (last on ties).

use crate::layout;
use crate::state::*;
use brickie_web_api::*;
use std::collections::BTreeSet;

/// Lane labels of the faded outside neighbours (see the server's level graph): they stay first
/// and last.
const INBOUND: &str = "inbound";
const OUTBOUND: &str = "outbound";

/// The (from, to) brick pairs the planned rules of `level` draw, each once, in rule order.
/// A selector `*` draws nothing; ghosts and self pairs are skipped.
pub fn unused_pairs(level: &LevelView) -> Vec<(usize, usize)> {
    let g = &level.graph;
    let ends = |sel: &str| -> Vec<usize> {
        (0..g.bricks.len())
            .filter(|&i| !g.bricks[i].ghost && sel != "*" && names(sel, &g.bricks[i]))
            .collect()
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for e in &level.unused_links {
        for a in ends(&e.from) {
            for b in ends(&e.to) {
                if a != b && seen.insert((a, b)) {
                    out.push((a, b));
                }
            }
        }
    }
    out
}

/// The layout input: one brick per graph brick (lanes reordered, see [`lane_order`]), the graph
/// edges in order, then `extra` edges (planned pairs) with ids that sort after them.
pub fn layout_input(g: &GraphView, extra: &[(usize, usize)]) -> layout::Input {
    let mut ends: Vec<(usize, usize)> = g.edges.iter().map(|e| (e.from, e.to)).collect();
    ends.extend_from_slice(extra);
    let order = lane_order(g, &ends);
    // new lane index of each old lane
    let mut at = vec![0; g.rows.len()];
    for (new, &old) in order.iter().enumerate() {
        at[old] = new;
    }
    let mut edges: Vec<layout::InEdge> = (0..g.edges.len())
        .map(|i| layout::InEdge {
            id: edge_key(g, i),
            a: g.edges[i].from,
            b: g.edges[i].to,
        })
        .collect();
    edges.extend(extra.iter().enumerate().map(|(i, &(a, b))| layout::InEdge {
        id: format!("~plan{i:04}"),
        a,
        b,
    }));
    layout::Input {
        bricks: (g.bricks.iter())
            .map(|b| layout::InBrick {
                id: b.name.clone(),
                row: at.get(b.row).copied().unwrap_or(b.row),
            })
            .collect(),
        rows: order.iter().map(|&r| g.rows[r].clone()).collect(),
        edges,
    }
}

/// The lane order (old lane indices): config order, with the untagged lane at the position
/// between the inbound and outbound lanes that minimises the summed lane span of `ends`.
pub fn lane_order(g: &GraphView, ends: &[(usize, usize)]) -> Vec<usize> {
    let all: Vec<usize> = (0..g.rows.len()).collect();
    let Some(u) = g.rows.iter().position(|r| r == UNTAGGED) else {
        return all;
    };
    let rest: Vec<usize> = all.into_iter().filter(|&r| r != u).collect();
    let lo = rest.iter().take_while(|&&r| g.rows[r] == INBOUND).count();
    let hi = rest.len()
        - rest
            .iter()
            .rev()
            .take_while(|&&r| g.rows[r] == OUTBOUND)
            .count();
    let row = |b: usize| g.bricks.get(b).map(|b| b.row);
    let span = |order: &[usize]| -> usize {
        let mut at = vec![0; g.rows.len()];
        for (i, &r) in order.iter().enumerate() {
            at[r] = i;
        }
        (ends.iter())
            .filter_map(|&(a, b)| Some(at[row(a)?].abs_diff(at[row(b)?])))
            .sum()
    };
    let with = |k: usize| {
        let mut o = rest.clone();
        o.insert(k, u);
        o
    };
    // Last wins ties: scan from the end and keep only strict improvements.
    (lo..=hi)
        .rev()
        .map(with)
        .fold(None::<(usize, Vec<usize>)>, |best, o| {
            let s = span(&o);
            match best {
                Some((b, _)) if b <= s => best,
                _ => Some((s, o)),
            }
        })
        .map_or(rest, |(_, o)| o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{GAPY_FREE, H, Pt};

    /// The hexagonal example: two adapter bricks, the untagged composition root, two core
    /// bricks, and the planned rules between adapter and core (allow down, deny up).
    fn hex() -> (GraphView, Vec<(usize, usize)>) {
        let b = |name: &str, row| BrickView {
            name: name.into(),
            row,
            ..Default::default()
        };
        let g = GraphView {
            rows: vec!["adapter".into(), UNTAGGED.into(), "core".into()],
            bricks: vec![
                b("adapters", 0),
                b("ui", 0),
                b("app", 1),
                b("application", 2),
                b("domain", 2),
            ],
            edges: Vec::new(),
        };
        let plan = vec![
            (0, 3),
            (0, 4),
            (1, 4),
            (1, 3),
            (3, 4),
            (3, 0),
            (3, 1),
            (4, 0),
            (4, 1),
        ];
        (g, plan)
    }

    #[test]
    fn untagged_lane_moves_out_of_the_way_and_its_gap_is_compact() {
        let (g, plan) = hex();
        let l = layout::layout(&layout_input(&g, &plan));
        let labels: Vec<&str> = l.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["adapter", "core", UNTAGGED]);
        // No edge crosses the gap above the untagged lane.
        assert_eq!(l.rows[2].y - l.rows[1].y, H + GAPY_FREE);
        // Each brick keeps its lane.
        assert_eq!(l.rows[2].ids, vec![2]);
    }

    #[test]
    fn untagged_lane_stays_between_the_lanes_it_links() {
        let (mut g, _) = hex();
        g.rows = vec!["adapter".into(), "core".into(), UNTAGGED.into()];
        for (i, r) in [0, 0, 2, 1, 1].into_iter().enumerate() {
            g.bricks[i].row = r;
        }
        // app uses every other brick: the middle keeps those edges shortest.
        let plan = vec![(2, 0), (2, 1), (2, 3), (2, 4)];
        let order = lane_order(&g, &plan);
        assert_eq!(order, vec![0, 2, 1]);
    }

    #[test]
    fn unused_routes_avoid_every_other_box() {
        let (g, plan) = hex();
        let l = layout::layout(&layout_input(&g, &plan));
        assert!(l.cross <= l.cross0);
        for (i, &(a, b)) in plan.iter().enumerate() {
            let r = l.path(i, &l.pos).expect("routed");
            for (k, bx) in l.pos.iter().enumerate().filter(|&(k, _)| k != a && k != b) {
                let hit = r.sample(4.0).iter().any(|q: &Pt| bx.contains(q, -1.0));
                assert!(!hit, "edge {a}->{b} crosses brick {k}");
            }
        }
    }
}
