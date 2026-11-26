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

/// The (from, to) brick pairs the drawn links of `level` connect (see [`drawn`]), each once,
/// in link order. Ghosts and self pairs are skipped.
pub(super) fn unused_pairs(level: &LevelView, editing: bool) -> Vec<(usize, usize)> {
    let g = &level.graph;
    let at = |end: &str| {
        let name = link_brick(end);
        (0..g.bricks.len()).find(|&i| !g.bricks[i].ghost && g.bricks[i].name == name)
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for e in level.links.iter().filter(|e| drawn(e, editing)) {
        if let (Some(a), Some(b)) = (at(&e.from), at(&e.to))
            && a != b
            && seen.insert((a, b))
        {
            out.push((a, b));
        }
    }
    for bl in level
        .boundary_links
        .iter()
        .filter(|b| drawn(&b.link, editing))
    {
        if let Some(c) = bl.child {
            let pair = if bl.inbound {
                (bl.ghost, c)
            } else {
                (c, bl.ghost)
            };
            if pair.0 != pair.1 && seen.insert(pair) {
                out.push(pair);
            }
        }
    }
    out
}

/// The layout input: one brick per graph brick (lanes reordered, see [`lane_order`]), the graph
/// edges in order (boundary edges skipped as self-edges when `to_frame` is true so hidden outside
/// edges never pollute crossing minimisation, box widths, ports, or channels), then `extra` edges
/// (planned pairs) with ids that sort after them.
pub fn layout_input(g: &GraphView, extra: &[(usize, usize)], to_frame: bool) -> layout::Input {
    let mut ends: Vec<(usize, usize)> = g
        .edges
        .iter()
        .filter(|e| !(to_frame && is_boundary(g, e)))
        .map(|e| (e.from, e.to))
        .collect();
    ends.extend_from_slice(extra);
    let order = lane_order(g, &ends);
    // new lane index of each old lane
    let mut at = vec![0; g.rows.len()];
    for (new, &old) in order.iter().enumerate() {
        at[old] = new;
    }
    let mut edges: Vec<layout::InEdge> = (0..g.edges.len())
        .map(|i| {
            let e = &g.edges[i];
            let (a, b) = if to_frame && is_boundary(g, e) {
                (0, 0)
            } else {
                (e.from, e.to)
            };
            layout::InEdge {
                id: edge_key(g, i),
                a,
                b,
            }
        })
        .collect();
    edges.extend(extra.iter().enumerate().map(|(i, &(a, b))| layout::InEdge {
        id: format!("~plan{i:04}"),
        a,
        b,
    }));
    layout::Input {
        bricks: (g.bricks.iter())
            .map(|b| {
                let text_w = brick_min_w(b);
                let (min_w, min_h) = if b.is_group && !b.ghost {
                    let (w, h) = group_box_size(b.group_members.len());
                    (Some(w.max(text_w)), Some(h))
                } else {
                    (Some(text_w), None)
                };
                layout::InBrick {
                    id: b.name.clone(),
                    row: at.get(b.row).copied().unwrap_or(b.row),
                    min_w,
                    min_h,
                }
            })
            .collect(),
        rows: order.iter().map(|&r| g.rows[r].clone()).collect(),
        edges,
    }
}

/// Minimum brick width so neither the header name (plus icons/badge) nor the sub-line
/// (`tag · N bricks · code/test`) ever overflows or gets clipped.
fn brick_min_w(b: &brickie_web_api::BrickView) -> f64 {
    let name = b.name.strip_prefix("root/").unwrap_or(&b.name);
    let name_pad = if b.independent { 74.0 } else { 58.0 };
    let name_w = name.chars().count() as f64 * 9.4 + name_pad;
    let sub = b.sub_line();
    let sub_pad = if b.composite() && !b.ghost {
        38.0
    } else {
        20.0
    };
    let sub_w = sub.chars().count() as f64 * 7.8 + sub_pad;
    name_w.max(sub_w).max(150.0).ceil()
}

/// The lane order (old lane indices): `inbound` ghost lanes stay first and `outbound` ghost lanes
/// stay last; inner tag lanes are ordered top-down by the directed dependency graph between lanes
/// (breaking cycles on dominant edge direction and ties by config order), then refined by
/// topological-order-preserving lane sifting to minimise summed vertical edge span.
pub fn lane_order(g: &GraphView, ends: &[(usize, usize)]) -> Vec<usize> {
    let n = g.rows.len();
    let all: Vec<usize> = (0..n).collect();
    if n <= 1 {
        return all;
    }
    let is_ghost_row = |r: usize| {
        g.bricks.iter().any(|b| b.row == r && b.ghost)
            && !g.bricks.iter().any(|b| b.row == r && !b.ghost)
    };
    let lo = all
        .iter()
        .take_while(|&&r| is_ghost_row(r) && g.rows[r] == INBOUND)
        .count();
    let hi = n - all
        .iter()
        .rev()
        .take_while(|&&r| is_ghost_row(r) && g.rows[r] == OUTBOUND)
        .count();
    if hi <= lo + 1 {
        return all;
    }
    let inner: Vec<usize> = all[lo..hi].to_vec();
    let k = inner.len();
    let mut pos_in = vec![None; n];
    for (i, &r) in inner.iter().enumerate() {
        pos_in[r] = Some(i);
    }
    let mut w = vec![vec![0usize; k]; k];
    for &(a, b) in ends {
        if let (Some(ra), Some(rb)) = (
            g.bricks.get(a).map(|x| x.row),
            g.bricks.get(b).map(|x| x.row),
        ) && let (Some(ia), Some(ib)) = (pos_in[ra], pos_in[rb])
            && ia != ib
        {
            w[ia][ib] += 1;
        }
    }
    // Net directed edges between inner lanes (dominant direction wins on bidirectional pairs).
    let mut dag = vec![vec![false; k]; k];
    let mut indeg = vec![0usize; k];
    let mut net_out = vec![0isize; k];
    for i in 0..k {
        for j in 0..k {
            if w[i][j] > w[j][i] {
                dag[i][j] = true;
                indeg[j] += 1;
                let diff = (w[i][j] - w[j][i]) as isize;
                net_out[i] += diff;
                net_out[j] -= diff;
            }
        }
    }
    // Deterministic topological order (Kahn; break cycles by highest net_out, ties by config order).
    let mut used = vec![false; k];
    let mut topo: Vec<usize> = Vec::with_capacity(k);
    for _ in 0..k {
        let next = (0..k)
            .filter(|&i| !used[i] && indeg[i] == 0)
            .min_by_key(|&i| (g.rows[inner[i]] == UNTAGGED, i))
            .unwrap_or_else(|| {
                (0..k)
                    .filter(|&i| !used[i])
                    .max_by(|&a, &b| net_out[a].cmp(&net_out[b]).then(b.cmp(&a)))
                    .unwrap()
            });
        used[next] = true;
        topo.push(next);
        for j in 0..k {
            if dag[next][j] && indeg[j] > 0 {
                indeg[j] -= 1;
            }
        }
    }
    // Refine by sliding each lane within its legal topological window to minimise total edge span.
    let span_of = |ord: &[usize]| -> usize {
        let mut at = vec![0usize; n];
        for (i, &r) in all[..lo].iter().enumerate() {
            at[r] = i;
        }
        for (i, &li) in ord.iter().enumerate() {
            at[inner[li]] = lo + i;
        }
        for (i, &r) in all[hi..].iter().enumerate() {
            at[r] = lo + ord.len() + i;
        }
        ends.iter()
            .filter_map(|&(a, b)| {
                let ra = g.bricks.get(a)?.row;
                let rb = g.bricks.get(b)?.row;
                Some(at[ra].abs_diff(at[rb]))
            })
            .sum()
    };
    for _pass in 0..4 {
        let mut improved = false;
        for li in 0..k {
            let cur_pos = topo.iter().position(|&x| x == li).unwrap();
            let mut base = topo.clone();
            base.remove(cur_pos);
            let min_pos = base
                .iter()
                .enumerate()
                .filter(|&(_, &x)| dag[x][li])
                .map(|(p, _)| p + 1)
                .max()
                .unwrap_or(0);
            let max_pos = base
                .iter()
                .enumerate()
                .filter(|&(_, &x)| dag[li][x])
                .map(|(p, _)| p)
                .min()
                .unwrap_or(base.len());
            let mut best_pos = cur_pos;
            let mut best_span = span_of(&topo);
            let is_untagged = g.rows[inner[li]] == UNTAGGED;
            for ins in min_pos..=max_pos {
                let mut cand = base.clone();
                cand.insert(ins, li);
                let s = span_of(&cand);
                if s < best_span || (is_untagged && s == best_span && ins > best_pos) {
                    best_span = s;
                    best_pos = ins;
                }
            }
            if best_pos != cur_pos {
                base.insert(best_pos, li);
                topo = base;
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }
    let mut out = Vec::with_capacity(n);
    out.extend_from_slice(&all[..lo]);
    out.extend(topo.into_iter().map(|li| inner[li]));
    out.extend_from_slice(&all[hi..]);
    out
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
        let l = layout::layout(&layout_input(&g, &plan, false));
        let labels: Vec<&str> = l.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["adapter", "core", UNTAGGED]);
        // No edge crosses the gap above the untagged lane.
        assert_eq!(l.rows[2].y - l.rows[1].y, H + GAPY_FREE);
        // Each brick keeps its lane.
        assert_eq!(l.rows[2].ids, vec![2]);
    }

    #[test]
    fn untagged_lane_moves_above_the_lanes_it_depends_on() {
        let (mut g, _) = hex();
        g.rows = vec!["adapter".into(), "core".into(), UNTAGGED.into()];
        for (i, r) in [0, 0, 2, 1, 1].into_iter().enumerate() {
            g.bricks[i].row = r;
        }
        // app uses every other brick: top-down order places the composition root at the top.
        let plan = vec![(2, 0), (2, 1), (2, 3), (2, 4)];
        let order = lane_order(&g, &plan);
        assert_eq!(order, vec![2, 0, 1]);
    }

    #[test]
    fn unused_routes_avoid_every_other_box() {
        let (g, plan) = hex();
        let l = layout::layout(&layout_input(&g, &plan, false));
        assert!(l.cross <= l.cross0);
        for (i, &(a, b)) in plan.iter().enumerate() {
            let r = l.path(i, &l.pos).expect("routed");
            for (k, bx) in l.pos.iter().enumerate().filter(|&(k, _)| k != a && k != b) {
                let hit = r.sample(4.0).iter().any(|q: &Pt| bx.contains(q, -1.0));
                assert!(!hit, "edge {a}->{b} crosses brick {k}");
            }
        }
    }

    #[test]
    fn frame_mode_omits_boundary_edges_from_layout_while_keeping_indices() {
        let g = GraphView {
            rows: vec!["inbound".into(), "core".into(), "outbound".into()],
            bricks: vec![
                BrickView {
                    name: "a".into(),
                    row: 1,
                    ..Default::default()
                },
                BrickView {
                    name: "b".into(),
                    row: 1,
                    ..Default::default()
                },
                BrickView {
                    name: "out_in".into(),
                    row: 0,
                    ghost: true,
                    ..Default::default()
                },
            ],
            edges: vec![
                EdgeView {
                    from: 2,
                    to: 0,
                    kind: "uses".into(),
                    ..Default::default()
                },
                EdgeView {
                    from: 0,
                    to: 1,
                    kind: "uses".into(),
                    ..Default::default()
                },
            ],
        };
        let inp = layout_input(&g, &[], true);
        assert_eq!(inp.edges.len(), 2);
        // Boundary edge 0 becomes a skipped self-edge; internal edge 1 stays intact.
        assert_eq!((inp.edges[0].a, inp.edges[0].b), (0, 0));
        assert_eq!((inp.edges[1].a, inp.edges[1].b), (0, 1));
        let l = layout::layout(&inp);
        assert!(l.routes[0].is_none());
        assert!(l.routes[1].is_some());
    }

    #[test]
    fn tag_lanes_order_top_down_by_dependencies() {
        // `adapter` and `ui` are declared after `service`, `api`, and `core`, but depend on them.
        // Topological lane ordering places consumers above their targets.
        let b = |name: &str, row| BrickView {
            name: name.into(),
            row,
            ..Default::default()
        };
        let g = GraphView {
            rows: vec![
                "service".into(),
                "api".into(),
                "core".into(),
                "adapter".into(),
                "ui".into(),
            ],
            bricks: vec![
                b("svc", 0),
                b("api", 1),
                b("core", 2),
                b("adp", 3),
                b("ui", 4),
            ],
            edges: Vec::new(),
        };
        let ends = vec![(0, 1), (1, 2), (3, 0), (3, 1), (3, 2), (4, 0)];
        let order = lane_order(&g, &ends);
        let pos = |r: usize| order.iter().position(|&x| x == r).unwrap();
        assert!(pos(3) < pos(0), "adapter above service: {order:?}");
        assert!(pos(4) < pos(0), "ui above service: {order:?}");
        assert!(
            pos(0) < pos(1) && pos(1) < pos(2),
            "service -> api -> core: {order:?}"
        );
    }
}
