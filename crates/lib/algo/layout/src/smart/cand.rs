//! Candidate routes of one edge: its current layered route plus one direct cubic per usable pair
//! of box sides.

use super::super::geometry::{bbox, bz_ctl, bz_port, bz_sample, poly_len};
use super::super::path::route_with;
use super::*;

/// One way to route an edge.
pub(super) struct Cand {
    /// The layered route the edge had before (always index 0, never capacity limited).
    pub(super) old: bool,
    pub(super) sa: Side,
    pub(super) sb: Side,
    /// Preference penalty of this side pair / route kind.
    pub(super) pen: f64,
    pub(super) pts: Vec<Pt>,
    /// Boxes (other than its own ends) the polyline touches.
    pub(super) boxh: usize,
    pub(super) len: f64,
    /// Bounding box `[x0, x1, y0, y1]` of `pts`.
    pub(super) bb: [f64; 4],
    /// Every second point of `pts` plus the last one (cheap crossing tests).
    pub(super) cp: Vec<Pt>,
}

impl Cand {
    /// A candidate over `pts` with `len`, `bb` and `cp` derived; `boxh` is the box-hit count
    /// the caller measured for `pts`.
    pub(super) fn new(old: bool, sa: Side, sb: Side, pen: f64, pts: Vec<Pt>, boxh: usize) -> Cand {
        let mut c = Cand {
            old,
            sa,
            sb,
            pen,
            pts: vec![],
            boxh,
            len: poly_len(&pts),
            bb: [0.0; 4],
            cp: vec![],
        };
        c.set_pts(pts);
        c
    }

    /// Replace the polyline and refresh its derived bounding box and coarse copy.
    pub(super) fn set_pts(&mut self, pts: Vec<Pt>) {
        self.bb = bbox(&pts);
        self.cp = coarse(&pts);
        self.pts = pts;
    }
}

/// Every second point of `p`, plus the last one.
pub(super) fn coarse(p: &[Pt]) -> Vec<Pt> {
    p.iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0 || *i == p.len() - 1)
        .map(|(_, q)| *q)
        .collect()
}

/// One end of a direct cubic: the box, the side it leaves by and the port offset on that side.
pub(super) type Anchor<'a> = (&'a Rect, Side, f64);

/// The direct cubic from `(a, sa, pa)` to `(b, sb, pb)` sampled every `step`.
pub(super) fn direct((a, sa, pa): Anchor, (b, sb, pb): Anchor, step: f64) -> Vec<Pt> {
    bz_sample(
        &bz_ctl(bz_port(a, sa, pa), sa, bz_port(b, sb, pb), sb),
        step,
    )
}

/// Number of boxes in `pos` that the polyline `pts` of an edge with ends `(a, b)` touches.
///
/// Other boxes count with a margin of 6 (any point inside the grown box); the two end boxes
/// count with margin -1 (strictly inside the shrunken box) and ignore the first and last three
/// points, which start and end on those boxes. Each box counts at most once.
pub(super) fn box_hits(pos: &[Rect], pts: &[Pt], (a, b): (usize, usize)) -> usize {
    let mut h = 0;
    for (i, n) in pos.iter().enumerate() {
        let own = i == a || i == b;
        let m = if own { -1.0 } else { 6.0 };
        let (lo, hi) = if own {
            (3, pts.len().saturating_sub(3))
        } else {
            (0, pts.len())
        };
        if pts.iter().take(hi).skip(lo).any(|q| n.contains(q, m)) {
            h += 1;
        }
    }
    h
}

/// Free distance from side `sd` of `a` to `b` along that side's normal (negative when `b` is
/// behind or overlapping).
fn gap_to(a: &Rect, sd: Side, b: &Rect) -> f64 {
    match sd {
        Side::R => b.x - (a.x + a.w),
        Side::L => a.x - (b.x + b.w),
        Side::B => b.y - (a.y + a.h),
        Side::T => a.y - (b.y + b.h),
    }
}

/// Penalty of leaving `a` by side `sa` and entering `b` by side `sb`, or `None` when either
/// side has less than 14 of room towards the other box (no space for the tangent).
///
/// Penalised: two horizontal sides when the boxes are further apart vertically (40), two
/// vertical sides when further apart horizontally by more than 1.6x (25), a mixed pair when the
/// smaller offset is under 30 (30, else 8); otherwise 0.
pub(super) fn side_penalty(a: &Rect, b: &Rect, sa: Side, sb: Side) -> Option<f64> {
    let (ga, gb) = (gap_to(a, sa, b), gap_to(b, sb, a));
    if ga < 14.0 || gb < 14.0 {
        return None;
    }
    let dx = (a.x.max(b.x) - (a.x + a.w).min(b.x + b.w)).max(0.0);
    let dy = (a.y.max(b.y) - (a.y + a.h).min(b.y + b.h)).max(0.0);
    Some(if sa.horizontal() && sb.horizontal() {
        if dy > dx { 40.0 } else { 0.0 }
    } else if !sa.horizontal() && !sb.horizontal() {
        if dx > dy * 1.6 { 25.0 } else { 0.0 }
    } else if dx.min(dy) < 30.0 {
        30.0
    } else {
        8.0
    })
}

/// The edge's current route as a candidate (penalty 45 for arcs, 12 otherwise).
fn old_cand(l: &Layout, e: usize) -> Cand {
    let ok = l.kinds[e].clone().unwrap_or(Kind::Bez);
    let pts = route_with(&ok, &l.po, &l.dpos, e, l.edges[e], &l.pos)
        .map(|r| r.sample(7.0))
        .unwrap_or_default();
    let sa = l.po.get(&(e, End::A)).map_or(Side::T, |p| p.side);
    let sb = l.po.get(&(e, End::B)).map_or(Side::T, |p| p.side);
    let pen = match ok {
        Kind::Arc { .. } => 45.0,
        _ => 12.0,
    };
    let boxh = box_hits(&l.pos, &pts, l.edges[e]);
    Cand::new(true, sa, sb, pen, pts, boxh)
}

/// Candidates of edge `e`: its current route (always first, kept even if it hits boxes) followed
/// by the box-free direct cubics between centred ports, in side order `T B L R` x `T B L R`.
pub(super) fn candidates(l: &Layout, e: usize) -> Vec<Cand> {
    let (ia, ib) = l.edges[e];
    let (a, b) = (&l.pos[ia], &l.pos[ib]);
    let mut list = vec![old_cand(l, e)];
    for sa in Side::ALL {
        for sb in Side::ALL {
            let Some(pen) = side_penalty(a, b, sa, sb) else {
                continue;
            };
            let pts = direct((a, sa, 0.0), (b, sb, 0.0), 7.0);
            let boxh = box_hits(&l.pos, &pts, l.edges[e]);
            list.push(Cand::new(false, sa, sb, pen, pts, boxh));
        }
    }
    list.into_iter().filter(|c| c.old || c.boxh == 0).collect()
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

    /// `n` points spread along y at x = 50 (inside a box at (0, 0, 100, 100)).
    fn column(n: usize) -> Vec<Pt> {
        (0..n).map(|i| p(50.0, 10.0 + i as f64)).collect()
    }

    #[test]
    fn coarse_keeps_even_points_and_the_last() {
        let pts: Vec<Pt> = (0..5).map(|i| p(i as f64, 0.0)).collect();
        let xs: Vec<f64> = coarse(&pts).iter().map(|q| q.x).collect();
        assert_eq!(xs, [0.0, 2.0, 4.0]);
        let pts: Vec<Pt> = (0..4).map(|i| p(i as f64, 0.0)).collect();
        let xs: Vec<f64> = coarse(&pts).iter().map(|q| q.x).collect();
        assert_eq!(xs, [0.0, 2.0, 3.0]);
        assert_eq!(coarse(&pts[..1]), vec![p(0.0, 0.0)]);
        assert!(coarse(&[]).is_empty());
    }

    #[test]
    fn set_pts_refreshes_the_derived_fields() {
        let mut c = Cand::new(false, Side::T, Side::B, 1.0, vec![], 0);
        c.set_pts(vec![p(1.0, 2.0), p(5.0, -1.0), p(3.0, 9.0)]);
        assert_eq!(c.bb, [1.0, 5.0, -1.0, 9.0]);
        assert_eq!(c.cp, vec![p(1.0, 2.0), p(3.0, 9.0)]);
        assert_eq!(c.pts.len(), 3);
    }

    #[test]
    fn new_derives_len_bb_and_cp_and_keeps_boxh() {
        let c = Cand::new(
            true,
            Side::L,
            Side::R,
            2.0,
            vec![p(0.0, 0.0), p(3.0, 4.0)],
            3,
        );
        assert_eq!((c.len, c.boxh, c.pen, c.old), (5.0, 3, 2.0, true));
        assert_eq!(c.bb, [0.0, 3.0, 0.0, 4.0]);
        assert_eq!(c.cp, vec![p(0.0, 0.0), p(3.0, 4.0)]);
    }

    #[test]
    fn direct_is_the_sampled_cubic_between_the_two_ports() {
        let (a, b) = (r(0.0, 0.0, 100.0, 50.0), r(300.0, 0.0, 100.0, 50.0));
        let pts = direct((&a, Side::R, 5.0), (&b, Side::L, -5.0), 7.0);
        let ctl = bz_ctl(
            bz_port(&a, Side::R, 5.0),
            Side::R,
            bz_port(&b, Side::L, -5.0),
            Side::L,
        );
        assert_eq!(pts, bz_sample(&ctl, 7.0));
        assert_eq!(pts[0], bz_port(&a, Side::R, 5.0));
    }

    #[test]
    fn box_hits_no_boxes_or_no_points() {
        assert_eq!(box_hits(&[], &column(10), (0, 1)), 0);
        assert_eq!(box_hits(&[r(0.0, 0.0, 100.0, 100.0)], &[], (5, 6)), 0);
    }

    #[test]
    fn box_hits_counts_each_other_box_once() {
        let pos = [r(0.0, 0.0, 100.0, 100.0), r(300.0, 0.0, 10.0, 10.0)];
        // ten points inside box 0 count once; box 1 is untouched
        assert_eq!(box_hits(&pos, &column(10), (8, 9)), 1);
        let both = [p(50.0, 50.0), p(305.0, 5.0)];
        assert_eq!(box_hits(&pos, &both, (8, 9)), 2);
    }

    #[test]
    fn box_hits_other_boxes_use_a_margin_of_six() {
        let pos = [r(0.0, 0.0, 100.0, 100.0)];
        let at = |x: f64| box_hits(&pos, &[p(x, 50.0)], (8, 9));
        assert_eq!(at(105.9), 1);
        assert_eq!(at(106.0), 0);
        assert_eq!(at(-5.9), 1);
        assert_eq!(at(-6.0), 0);
        let at_y = |y: f64| box_hits(&pos, &[p(50.0, y)], (8, 9));
        assert_eq!(at_y(105.9), 1);
        assert_eq!(at_y(-6.0), 0);
    }

    #[test]
    fn box_hits_own_boxes_ignore_the_first_and_last_three_points() {
        let pos = [r(0.0, 0.0, 100.0, 100.0)];
        // 6 points all inside the own box: lo = 3, hi = 3, nothing is looked at
        assert_eq!(box_hits(&pos, &column(6), (0, 1)), 0);
        // 7 points: only index 3 is looked at, and it is inside
        assert_eq!(box_hits(&pos, &column(7), (0, 1)), 1);
        // the same applies when the box is the second end
        assert_eq!(box_hits(&pos, &column(6), (1, 0)), 0);
        assert_eq!(box_hits(&pos, &column(7), (1, 0)), 1);
        // fewer than three points: saturating, nothing is looked at
        assert_eq!(box_hits(&pos, &column(2), (0, 1)), 0);
    }

    #[test]
    fn box_hits_own_boxes_use_a_negative_margin() {
        let pos = [r(0.0, 0.0, 100.0, 100.0)];
        let mut pts = vec![p(-50.0, 50.0); 3];
        pts.push(p(1.5, 50.0)); // 1.5 inside the edge: hit
        pts.extend(vec![p(-50.0, 50.0); 3]);
        assert_eq!(box_hits(&pos, &pts, (0, 1)), 1);
        pts[3] = p(1.0, 50.0); // the box is shrunk by 1: not strictly inside
        assert_eq!(box_hits(&pos, &pts, (0, 1)), 0);
        pts[3] = p(-0.5, 50.0); // outside, and no outer margin for own boxes
        assert_eq!(box_hits(&pos, &pts, (0, 1)), 0);
    }

    #[test]
    fn side_penalty_needs_fourteen_of_room_on_both_sides() {
        let a = r(0.0, 0.0, 50.0, 50.0);
        // R of a faces b: gap 14 is allowed, 13.9 is not
        assert!(side_penalty(&a, &r(64.0, 100.0, 50.0, 50.0), Side::R, Side::T).is_some());
        assert!(side_penalty(&a, &r(63.9, 100.0, 50.0, 50.0), Side::R, Side::T).is_none());
        // the second side lacks room: T of b towards a needs b.y - 50 >= 14
        assert!(side_penalty(&a, &r(100.0, 63.9, 50.0, 50.0), Side::R, Side::T).is_none());
        // a side facing away has a negative gap
        assert!(side_penalty(&a, &r(100.0, 100.0, 50.0, 50.0), Side::L, Side::T).is_none());
    }

    #[test]
    fn side_penalty_two_horizontal_sides() {
        let a = r(0.0, 0.0, 100.0, 50.0);
        // further apart vertically than horizontally: 40
        let b = r(130.0, 200.0, 100.0, 50.0);
        assert_eq!(side_penalty(&a, &b, Side::R, Side::L), Some(40.0));
        // side by side: 0
        let b = r(200.0, 0.0, 100.0, 50.0);
        assert_eq!(side_penalty(&a, &b, Side::R, Side::L), Some(0.0));
    }

    #[test]
    fn side_penalty_two_vertical_sides() {
        let a = r(0.0, 0.0, 50.0, 50.0);
        // dx 250 > 1.6 * dy 50: 25
        assert_eq!(
            side_penalty(&a, &r(300.0, 100.0, 50.0, 50.0), Side::B, Side::T),
            Some(25.0)
        );
        // dx 10 against dy 50: 0
        assert_eq!(
            side_penalty(&a, &r(60.0, 100.0, 50.0, 50.0), Side::B, Side::T),
            Some(0.0)
        );
        // dx exactly 1.6 * dy is not penalised
        assert_eq!(
            side_penalty(&a, &r(130.0, 100.0, 50.0, 50.0), Side::B, Side::T),
            Some(0.0)
        );
    }

    #[test]
    fn side_penalty_mixed_sides() {
        let a = r(0.0, 0.0, 50.0, 50.0);
        // both offsets 20 (< 30): 30
        assert_eq!(
            side_penalty(&a, &r(70.0, 70.0, 50.0, 50.0), Side::R, Side::T),
            Some(30.0)
        );
        // both offsets 50: 8
        assert_eq!(
            side_penalty(&a, &r(100.0, 100.0, 50.0, 50.0), Side::R, Side::T),
            Some(8.0)
        );
        // the smaller of the two decides, in either side order
        assert_eq!(
            side_penalty(&a, &r(70.0, 200.0, 50.0, 50.0), Side::R, Side::T),
            Some(30.0)
        );
        assert_eq!(
            side_penalty(&a, &r(70.0, 200.0, 50.0, 50.0), Side::B, Side::L),
            Some(30.0)
        );
    }

    /// Three bricks in three rows; edge 0 goes from brick 0 to brick 2.
    fn three_row_layout() -> Layout {
        let bricks = (0..3)
            .map(|i| InBrick {
                id: format!("b{i}"),
                row: i,
            })
            .collect();
        let input = Input {
            bricks,
            rows: vec!["r0".into(), "r1".into(), "r2".into()],
            edges: vec![InEdge {
                id: "e".into(),
                a: 0,
                b: 2,
            }],
        };
        let mut l = layout(&input);
        l.pos = vec![
            r(0.0, 0.0, 100.0, 50.0),
            r(0.0, 150.0, 100.0, 50.0),
            r(0.0, 300.0, 100.0, 50.0),
        ];
        l
    }

    #[test]
    fn candidates_start_with_the_old_route_then_free_direct_cubics() {
        let mut l = three_row_layout();
        l.pos[1] = r(400.0, 150.0, 100.0, 50.0); // out of the way
        let list = candidates(&l, 0);
        assert!(list[0].old);
        assert!(list[1..].iter().all(|c| !c.old));
        assert!(list.len() > 1);
        let (a, b) = (&l.pos[0], &l.pos[2]);
        let mut last = None;
        for c in &list[1..] {
            assert_eq!(c.boxh, 0);
            assert_eq!(side_penalty(a, b, c.sa, c.sb), Some(c.pen));
            let at = |s: Side| Side::ALL.iter().position(|x| *x == s).unwrap();
            let key = (at(c.sa), at(c.sb));
            assert!(last.is_none_or(|k| k < key), "side order T B L R x T B L R");
            last = Some(key);
            assert!(c.len > 0.0 && c.pts.len() > 2);
            assert_eq!(c.cp, coarse(&c.pts));
            assert_eq!(c.bb, super::super::geometry::bbox(&c.pts));
        }
        // the straight bottom-to-top cubic is free here
        assert!(
            list.iter()
                .any(|c| !c.old && c.sa == Side::B && c.sb == Side::T)
        );
    }

    #[test]
    fn candidates_drop_direct_cubics_that_hit_a_box() {
        let l = three_row_layout(); // brick 1 sits between 0 and 2
        let list = candidates(&l, 0);
        assert!(list[0].old);
        assert!(
            !list
                .iter()
                .any(|c| !c.old && c.sa == Side::B && c.sb == Side::T)
        );
        assert!(list.iter().all(|c| c.old || c.boxh == 0));
    }

    #[test]
    fn candidates_penalise_the_old_route_by_kind() {
        let mut l = three_row_layout();
        l.kinds[0] = Some(Kind::Arc {
            top: true,
            level: 1,
        });
        assert_eq!(candidates(&l, 0)[0].pen, 45.0);
        l.kinds[0] = Some(Kind::Bez);
        assert_eq!(candidates(&l, 0)[0].pen, 12.0);
        l.kinds[0] = None;
        assert_eq!(candidates(&l, 0)[0].pen, 12.0);
    }

    #[test]
    fn candidates_old_route_uses_the_current_ports() {
        let mut l = three_row_layout();
        l.kinds[0] = Some(Kind::Bez);
        l.po.insert(
            (0, End::A),
            Port {
                side: Side::R,
                off: 0.0,
            },
        );
        l.po.insert(
            (0, End::B),
            Port {
                side: Side::L,
                off: 0.0,
            },
        );
        let old = &candidates(&l, 0)[0];
        assert_eq!((old.sa, old.sb), (Side::R, Side::L));
        l.po.clear();
        let old = &candidates(&l, 0)[0];
        assert_eq!((old.sa, old.sb), (Side::T, Side::T));
    }
}
