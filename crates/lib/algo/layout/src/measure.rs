//! Layout quality on sampled paths (`mkItem`, `measureItems`, `scoreLayout`): crossings between
//! edges, edge-through-box hits and direction changes. Used by the tests.

use super::geometry::{bb_apart, bbox};
use super::path::route_path;
use super::*;

/// One edge's sampled path for [`measure_items`] (`mkItem`).
#[derive(Debug, Clone)]
pub struct Item {
    pub a: usize,
    pub b: usize,
    pub pts: Vec<Pt>,
    pub bb: [f64; 4],
}

/// `mkItem`.
pub fn mk_item(a: usize, b: usize, pts: Vec<Pt>) -> Item {
    Item {
        a,
        b,
        bb: bbox(&pts),
        pts,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Metrics {
    pub edges: usize,
    pub crossings: usize,
    pub box_hits: usize,
    pub bends: usize,
}

/// Cross product of `(b - a)` and `(c - a)`: the signed side of `c` relative to line `a -> b`.
fn side(a: Pt, b: Pt, c: Pt) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// True when segments `p-q` and `r-s` properly cross: each segment's endpoints lie strictly on
/// opposite sides of the other. Collinear, touching and shared-endpoint cases are not crossings.
fn seg_cross(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    side(p, q, r) * side(p, q, s) < 0.0 && side(r, s, p) * side(r, s, q) < 0.0
}

/// Crossings between the paths of `a` and `b`. Zero when their bounding boxes are apart. A
/// crossing is ignored when `a`'s segment start lies within radius 10 of a node both edges share
/// (the port area), and crossings on `a` segments at most 2 indices after the previous counted
/// one are merged into it.
fn pair_crossings(a: &Item, b: &Item, nodes: &[Rect]) -> usize {
    if bb_apart(&a.bb, &b.bb, 0.0) {
        return 0;
    }
    let shared: Vec<usize> = [a.a, a.b]
        .into_iter()
        .filter(|x| *x == b.a || *x == b.b)
        .collect();
    let (mut count, mut last) = (0, -9i64);
    for x in 0..a.pts.len().saturating_sub(1) {
        for y in 0..b.pts.len().saturating_sub(1) {
            if !seg_cross(a.pts[x], a.pts[x + 1], b.pts[y], b.pts[y + 1]) {
                continue;
            }
            if shared.iter().any(|&n| nodes[n].contains(&a.pts[x], 10.0)) || x as i64 - last <= 2 {
                continue;
            }
            last = x as i64;
            count += 1;
        }
    }
    count
}

/// Number of nodes, other than the edge's own endpoints, whose box (shrunk by 2) contains at
/// least one sampled point of `item`. Each node counts at most once.
fn box_hits(item: &Item, nodes: &[Rect]) -> usize {
    nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != item.a && *i != item.b)
        .filter(|(_, n)| !bb_apart(&item.bb, &n.bb(), 0.0))
        .filter(|(_, n)| item.pts.iter().any(|q| n.contains(q, -2.0)))
        .count()
}

/// Direction of a step `d`: 0.0 when `|d| < 0.4`, otherwise its sign.
fn dir(d: f64) -> f64 {
    if d.abs() < 0.4 { 0.0 } else { d.signum() }
}

/// Feeds direction `g` on one axis; true when it reverses the last non-zero direction `last`.
fn turn(last: &mut f64, g: f64) -> bool {
    if g == 0.0 {
        return false;
    }
    let flipped = *last != 0.0 && g != *last;
    *last = g;
    flipped
}

/// Direction changes along `pts`, counted per axis. Steps shorter than 0.4 on an axis are
/// ignored for that axis; a change is a non-zero direction opposite to the previous non-zero
/// one. Fewer than two points give 0.
fn bends(pts: &[Pt]) -> usize {
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let mut n = 0;
    for w in pts.windows(2) {
        n += usize::from(turn(&mut sx, dir(w[1].x - w[0].x)));
        n += usize::from(turn(&mut sy, dir(w[1].y - w[0].y)));
    }
    n
}

/// `measureItems`: crossings between edges, edge-through-box hits, direction changes.
pub fn measure_items(es: &[Item], nodes: &[Rect]) -> Metrics {
    let crossings = (0..es.len())
        .flat_map(|i| (i + 1..es.len()).map(move |j| (i, j)))
        .map(|(i, j)| pair_crossings(&es[i], &es[j], nodes))
        .sum();
    Metrics {
        edges: es.len(),
        crossings,
        box_hits: es.iter().map(|a| box_hits(a, nodes)).sum(),
        bends: es.iter().map(|a| bends(&a.pts)).sum(),
    }
}

/// `scoreLayout` plus detail: metrics of the edges in `which` (edge indices) on layout `l`.
pub fn score_layout(l: &Layout, which: &[usize]) -> Metrics {
    let items: Vec<Item> = which
        .iter()
        .filter_map(|&e| {
            let r = route_path(l, e, &l.pos)?;
            let (a, b) = l.edges[e];
            Some(mk_item(a, b, r.sample(6.0)))
        })
        .collect();
    measure_items(&items, &l.pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Pt {
        Pt { x, y }
    }

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    fn path(v: &[(f64, f64)]) -> Vec<Pt> {
        v.iter().map(|&(x, y)| p(x, y)).collect()
    }

    #[test]
    fn seg_cross_proper_x() {
        assert!(seg_cross(p(0., 0.), p(10., 10.), p(0., 10.), p(10., 0.)));
    }

    #[test]
    fn seg_cross_parallel_and_collinear() {
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(0., 5.), p(10., 5.)));
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(2., 0.), p(8., 0.)));
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(5., 0.), p(15., 0.)));
    }

    #[test]
    fn seg_cross_touching_endpoints_not_crossing() {
        // shared endpoint
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(10., 0.), p(10., 10.)));
        // T junction: one endpoint on the other segment's interior
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(5., 0.), p(5., 10.)));
    }

    #[test]
    fn seg_cross_lines_cross_but_segments_do_not() {
        // r-s straddles the line of p-q but stops short of p-q itself.
        assert!(!seg_cross(p(0., 0.), p(10., 0.), p(20., -5.), p(20., 5.)));
        // p-q straddles the line of r-s but stops short of it.
        assert!(!seg_cross(p(0., -5.), p(0., 5.), p(5., 0.), p(10., 0.)));
    }

    #[test]
    fn seg_cross_is_symmetric() {
        let (a, b, c, d) = (p(0., 0.), p(10., 4.), p(3., 8.), p(6., -3.));
        assert_eq!(seg_cross(a, b, c, d), seg_cross(c, d, a, b));
        assert!(seg_cross(a, b, c, d));
    }

    fn far_nodes() -> Vec<Rect> {
        vec![r(-500., -500., 10., 10.), r(-500., -400., 10., 10.)]
    }

    fn item(a: usize, b: usize, v: &[(f64, f64)]) -> Item {
        mk_item(a, b, path(v))
    }

    #[test]
    fn pair_crossings_simple_x() {
        let a = item(0, 1, &[(0., 0.), (10., 10.)]);
        let b = item(0, 1, &[(0., 10.), (10., 0.)]);
        // shared nodes 0 and 1 are far from the crossing start, so it counts.
        assert_eq!(pair_crossings(&a, &b, &far_nodes()), 1);
    }

    #[test]
    fn pair_crossings_apart_bboxes_and_short_paths() {
        let a = item(0, 1, &[(0., 0.), (10., 10.)]);
        let b = item(2, 3, &[(100., 0.), (110., 10.)]);
        assert_eq!(pair_crossings(&a, &b, &far_nodes()), 0);
        let one = item(2, 3, &[(5., 5.)]);
        assert_eq!(pair_crossings(&a, &one, &far_nodes()), 0);
        let empty = item(2, 3, &[]);
        assert_eq!(pair_crossings(&empty, &a, &far_nodes()), 0);
    }

    #[test]
    fn pair_crossings_shared_node_radius_skips() {
        let a = item(0, 1, &[(0., 0.), (10., 10.)]);
        let b = item(0, 2, &[(0., 10.), (10., 0.)]);
        // a's segment start (0,0) is inside node 0 grown by 10: skipped.
        let near = vec![
            r(0., 0., 4., 4.),
            r(-500., 0., 4., 4.),
            r(-500., 50., 4., 4.),
        ];
        assert_eq!(pair_crossings(&a, &b, &near), 0);
        // Without a shared node the same geometry counts.
        let c = item(2, 3, &[(0., 10.), (10., 0.)]);
        assert_eq!(pair_crossings(&a, &c, &near), 1);
    }

    #[test]
    fn pair_crossings_shared_radius_is_strict() {
        let a = item(0, 1, &[(0., 0.), (10., 10.)]);
        let b = item(0, 2, &[(0., 10.), (10., 0.)]);
        // Node 0 is [20,24]^2; (0,0) lies exactly 20 away: outside grown box (x > 10 fails).
        let n = vec![
            r(20., 20., 4., 4.),
            r(-500., 0., 4., 4.),
            r(-500., 50., 4., 4.),
        ];
        assert_eq!(pair_crossings(&a, &b, &n), 1);
        // Exactly on the grown boundary (x == n.x - 10) is not inside.
        let edge = vec![
            r(10., 10., 4., 4.),
            r(-500., 0., 4., 4.),
            r(-500., 50., 4., 4.),
        ];
        assert_eq!(pair_crossings(&a, &b, &edge), 1);
    }

    #[test]
    fn pair_crossings_gap_merges_nearby_segments() {
        // a zig-zags across b's long horizontal segment on consecutive segments.
        let a = item(
            0,
            1,
            &[
                (0., -5.),
                (1., 5.),
                (2., -5.),
                (3., 5.),
                (4., -5.),
                (5., 5.),
            ],
        );
        let b = item(2, 3, &[(-10., 0.), (20., 0.)]);
        // crossings at x = 0,1,2,3,4: counted at 0 and 3 (gap > 2).
        assert_eq!(pair_crossings(&a, &b, &far_nodes()), 2);
    }

    #[test]
    fn pair_crossings_gap_boundary() {
        // Crossings on a segments 0 and 2 (gap 2 -> skipped), 0 and 3 (gap 3 -> counted).
        let two = item(0, 1, &[(0., -5.), (1., 5.), (2., 5.), (3., -5.), (4., -5.)]);
        let b = item(2, 3, &[(-10., 0.), (20., 0.)]);
        // crossings on segments 0 (up) and 2 (down).
        assert_eq!(pair_crossings(&two, &b, &far_nodes()), 1);
        let three = item(
            0,
            1,
            &[
                (0., -5.),
                (1., 5.),
                (2., 5.),
                (3., 5.),
                (4., -5.),
                (5., -5.),
            ],
        );
        // crossings on segments 0 and 3.
        assert_eq!(pair_crossings(&three, &b, &far_nodes()), 2);
    }

    #[test]
    fn box_hits_counts_other_nodes_only() {
        let nodes = vec![
            r(0., 0., 10., 10.),
            r(40., 0., 10., 10.),
            r(20., 0., 10., 10.),
        ];
        let it = item(0, 1, &[(5., 5.), (25., 5.), (45., 5.)]);
        // own endpoints 0 and 1 skipped; node 2 contains (25,5).
        assert_eq!(box_hits(&it, &nodes), 1);
    }

    #[test]
    fn box_hits_each_node_once_and_multiple_nodes() {
        let nodes = vec![
            r(100., 100., 5., 5.),
            r(100., 200., 5., 5.),
            r(10., 0., 10., 10.),
            r(30., 0., 10., 10.),
        ];
        let it = item(0, 1, &[(12., 5.), (15., 5.), (18., 5.), (35., 5.)]);
        assert_eq!(box_hits(&it, &nodes), 2);
    }

    #[test]
    fn box_hits_margin_is_shrunk_by_two() {
        let nodes = vec![
            r(100., 100., 5., 5.),
            r(100., 200., 5., 5.),
            r(10., 0., 10., 10.),
        ];
        // x = 12 is exactly n.x + 2: not strictly inside.
        let edge = item(0, 1, &[(12., 5.), (12., 6.)]);
        assert_eq!(box_hits(&edge, &nodes), 0);
        let inside = item(0, 1, &[(12.5, 5.), (12.5, 6.)]);
        assert_eq!(box_hits(&inside, &nodes), 1);
    }

    #[test]
    fn box_hits_bbox_prefilter_and_degenerate_paths() {
        let nodes = vec![
            r(100., 100., 5., 5.),
            r(100., 200., 5., 5.),
            r(10., 0., 10., 10.),
        ];
        let away = item(0, 1, &[(50., 5.), (60., 5.)]);
        assert_eq!(box_hits(&away, &nodes), 0);
        let empty = item(0, 1, &[]);
        assert_eq!(box_hits(&empty, &nodes), 0);
        let one = item(0, 1, &[(15., 5.)]);
        assert_eq!(box_hits(&one, &nodes), 1);
    }

    #[test]
    fn bends_empty_and_single_point() {
        assert_eq!(bends(&[]), 0);
        assert_eq!(bends(&path(&[(1., 2.)])), 0);
    }

    #[test]
    fn bends_straight_and_monotone() {
        assert_eq!(bends(&path(&[(0., 0.), (5., 0.), (10., 0.)])), 0);
        assert_eq!(bends(&path(&[(0., 0.), (5., 5.), (10., 12.)])), 0);
    }

    #[test]
    fn bends_each_axis_counts() {
        // x reverses once.
        assert_eq!(bends(&path(&[(0., 0.), (5., 0.), (0., 0.)])), 1);
        // y reverses once.
        assert_eq!(bends(&path(&[(0., 0.), (0., 5.), (0., 0.)])), 1);
        // both axes reverse on the same step.
        assert_eq!(bends(&path(&[(0., 0.), (5., 5.), (0., 0.)])), 2);
        // zig-zag: three reversals.
        assert_eq!(
            bends(&path(&[(0., 0.), (5., 0.), (0., 0.), (5., 0.), (0., 0.)])),
            3
        );
    }

    #[test]
    fn bends_threshold_is_inclusive_of_point_four() {
        // |dx| == 0.4 is a real step (not < 0.4); 0.39 is ignored.
        assert_eq!(bends(&path(&[(0., 0.), (5., 0.), (4.6, 0.)])), 1);
        assert_eq!(bends(&path(&[(0., 0.), (5., 0.), (4.61, 0.)])), 0);
        assert_eq!(bends(&path(&[(0., 0.), (0., 5.), (0., 4.6)])), 1);
        assert_eq!(bends(&path(&[(0., 0.), (0., 5.), (0., 4.61)])), 0);
    }

    #[test]
    fn bends_small_steps_keep_previous_direction() {
        // The tiny backwards step is dropped, so the later backwards step is the first reversal.
        assert_eq!(bends(&path(&[(0., 0.), (5., 0.), (4.8, 0.), (0., 0.)])), 1);
    }

    #[test]
    fn measure_items_wires_the_parts() {
        let nodes = vec![
            r(100., 100., 5., 5.),
            r(100., 200., 5., 5.),
            r(10., 0., 10., 10.),
        ];
        let a = item(0, 1, &[(0., 0.), (15., 5.), (30., 30.)]);
        let b = item(0, 1, &[(0., 30.), (30., 0.)]);
        let m = measure_items(&[a, b], &nodes);
        assert_eq!(m.edges, 2);
        assert_eq!(m.crossings, 1);
        assert_eq!(m.box_hits, 1);
        assert_eq!(m.bends, 0);
        assert_eq!(measure_items(&[], &nodes), Metrics::default());
    }
}
