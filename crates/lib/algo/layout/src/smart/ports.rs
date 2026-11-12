//! Port spreading for the chosen candidates and the repair rounds that follow it.

use super::cand::{box_hits, direct};
use super::*;

/// One edge end placed on a side: where its route leaves (`q`, `REFK` points in) and where the
/// other end of the route is (`far`).
struct Pl {
    key: (usize, End),
    q: Pt,
    far: Pt,
}

/// Largest distance between neighbouring ports on one side.
const MAX_STEP: f64 = 26.0;

impl Router<'_> {
    /// Edge ends of the chosen candidates grouped per (brick, side), in `eids` order.
    fn placements(&self, cands: &[Vec<Cand>], choice: &[usize]) -> HashMap<(usize, Side), Vec<Pl>> {
        let mut pl: HashMap<(usize, Side), Vec<Pl>> = HashMap::new();
        for &e in self.eids {
            let c = &cands[e][choice[e]];
            let n = c.pts.len();
            let k = n.saturating_sub(1).min(REFK);
            let (first, last) = (c.pts[0], c.pts[n - 1]);
            let (ea, eb) = self.edges[e];
            let ends = [
                (End::A, ea, c.sa, c.pts[k], last),
                (End::B, eb, c.sb, c.pts[n - 1 - k], first),
            ];
            for (end, brick, sd, q, far) in ends {
                let key = (e, end);
                pl.entry((brick, sd)).or_default().push(Pl { key, q, far });
            }
        }
        pl
    }

    /// Sort the ends of one side by where they go: by `q` then `far` along the side's axis (x on
    /// top/bottom sides, y on left/right ones). Left/right sides break remaining ties by edge id
    /// then end (A before B); top/bottom ties keep their input order (stable sort), so they rely
    /// on `placements` yielding ends in the deterministic `eids` order.
    fn sort_side(&self, list: &mut [Pl], sd: Side) {
        let hz = !sd.horizontal();
        let key = |x: &Pl| (self.ids[x.key.0].as_str(), x.key.1 == End::B);
        list.sort_by(|p, q| {
            if hz {
                p.q.x.total_cmp(&q.q.x).then(p.far.x.total_cmp(&q.far.x))
            } else {
                p.q.y
                    .total_cmp(&q.q.y)
                    .then(p.far.y.total_cmp(&q.far.y))
                    .then(key(p).cmp(&key(q)))
            }
        });
    }

    /// Spread ports per (brick, side), ordered by the direction the edge leaves in: evenly over
    /// the side minus `PAD` at both ends, at most 26 apart, centred (a single port gets offset 0).
    /// Returns the port of both ends of every routed edge, for the chosen candidates.
    pub(super) fn spread_ports(
        &self,
        cands: &[Vec<Cand>],
        choice: &[usize],
    ) -> HashMap<(usize, End), Port> {
        let mut po = HashMap::new();
        for ((n, sd), mut list) in self.placements(cands, choice) {
            self.sort_side(&mut list, sd);
            let cnt = list.len() as f64;
            let st = if list.len() > 1 {
                ((self.pos[n].side_len(sd) - 2.0 * PAD) / (cnt - 1.0)).min(MAX_STEP)
            } else {
                0.0
            };
            for (i, p) in list.iter().enumerate() {
                let off = (i as f64 - (cnt - 1.0) / 2.0) * st;
                po.insert(p.key, Port { side: sd, off });
            }
        }
        po
    }

    /// Re-sample direct candidate `ci` of edge `e` with its spread ports `po`. A route that now
    /// touches a box resets the edge to its layered route (index 0) and returns true; otherwise
    /// the candidate's polyline is refreshed.
    fn repair(
        &self,
        e: usize,
        po: &HashMap<(usize, End), Port>,
        cands: &mut [Vec<Cand>],
        choice: &mut [usize],
    ) -> bool {
        let ci = choice[e];
        if cands[e][ci].old {
            return false;
        }
        let (sa, sb) = (cands[e][ci].sa, cands[e][ci].sb);
        let (pa, pb) = (po[&(e, End::A)].off, po[&(e, End::B)].off);
        let (ea, eb) = self.edges[e];
        let pts = direct((&self.pos[ea], sa, pa), (&self.pos[eb], sb, pb), 4.0);
        if box_hits(self.pos, &pts, self.edges[e]) > 0 {
            choice[e] = 0;
            true
        } else {
            cands[e][ci].set_pts(pts);
            false
        }
    }

    /// Up to 8 repair rounds: spread the ports, re-sample every chosen direct route with them and
    /// reset those that hit a box to their layered route; stop when a round changes nothing.
    pub(super) fn settle(&self, cands: &mut [Vec<Cand>], choice: &mut [usize]) {
        for _round in 0..8 {
            let po = self.spread_ports(cands, choice);
            let mut changed = false;
            for &e in self.eids {
                changed |= self.repair(e, &po, cands, choice);
            }
            if !changed {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{bbox, bz_port};
    use crate::smart::cand::{Cand, coarse};

    fn p(x: f64, y: f64) -> Pt {
        Pt { x, y }
    }

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    fn cd(old: bool, sa: Side, sb: Side, pts: &[Pt]) -> Cand {
        Cand::new(old, sa, sb, 0.0, pts.to_vec(), 0)
    }

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// A two-point route from `from` to `to`, both ends on the top sides.
    fn seg(from: (f64, f64), to: (f64, f64)) -> Vec<Cand> {
        vec![cd(
            false,
            Side::T,
            Side::T,
            &[p(from.0, from.1), p(to.0, to.1)],
        )]
    }

    fn offs(po: &HashMap<(usize, End), Port>, keys: &[(usize, End)]) -> Vec<f64> {
        keys.iter().map(|k| po[k].off).collect()
    }

    #[test]
    fn a_single_port_has_offset_zero_on_the_chosen_side() {
        let pos = [r(0.0, 0.0, 100.0, 50.0), r(300.0, 0.0, 100.0, 50.0)];
        let edges = [(0, 1)];
        let ids = ids(&["e"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let cands = vec![vec![cd(
            false,
            Side::R,
            Side::L,
            &[p(100.0, 25.0), p(300.0, 25.0)],
        )]];
        let po = rt.spread_ports(&cands, &[0]);
        assert_eq!(po.len(), 2);
        assert_eq!(
            po[&(0, End::A)],
            Port {
                side: Side::R,
                off: 0.0
            }
        );
        assert_eq!(
            po[&(0, End::B)],
            Port {
                side: Side::L,
                off: 0.0
            }
        );
    }

    #[test]
    fn ports_spread_evenly_centred_with_a_step_of_at_most_26() {
        // top side 400 wide: usable 380, 3 ports -> step min(190, 26) = 26
        let pos = [
            r(0.0, 0.0, 400.0, 50.0),
            r(0.0, 200.0, 400.0, 50.0),
            r(500.0, 0.0, 400.0, 50.0),
        ];
        let edges = [(0, 1), (0, 2), (0, 1)];
        let ids = ids(&["a", "b", "c"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1, 2],
            ids: &ids,
        };
        let cands = vec![
            seg((10.0, 0.0), (10.0, 90.0)),
            seg((20.0, 0.0), (20.0, 90.0)),
            seg((30.0, 0.0), (30.0, 90.0)),
        ];
        let po = rt.spread_ports(&cands, &[0, 0, 0]);
        assert_eq!(
            offs(&po, &[(0, End::A), (1, End::A), (2, End::A)]),
            [-26.0, 0.0, 26.0]
        );
    }

    #[test]
    fn a_narrow_side_squeezes_the_step_below_26() {
        // 50 wide: usable 30, 3 ports -> step 15
        let pos = [r(0.0, 0.0, 50.0, 50.0), r(0.0, 200.0, 400.0, 50.0)];
        let edges = [(0, 1), (0, 1), (0, 1)];
        let ids = ids(&["a", "b", "c"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1, 2],
            ids: &ids,
        };
        let cands = vec![
            seg((10.0, 0.0), (0.0, 90.0)),
            seg((20.0, 0.0), (0.0, 90.0)),
            seg((30.0, 0.0), (0.0, 90.0)),
        ];
        let po = rt.spread_ports(&cands, &[0, 0, 0]);
        assert_eq!(
            offs(&po, &[(0, End::A), (1, End::A), (2, End::A)]),
            [-15.0, 0.0, 15.0]
        );
        // two ports on 400 wide: 380 -> capped at 26 -> -13, 13
        let rt2 = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let po = rt2.spread_ports(&cands, &[0, 0, 0]);
        assert_eq!(offs(&po, &[(0, End::B), (1, End::B)]), [-13.0, 13.0]);
    }

    #[test]
    fn top_and_bottom_sides_order_by_x_of_the_leaving_point_then_far_end() {
        let pos = [
            r(0.0, 0.0, 100.0, 50.0),
            r(0.0, 200.0, 400.0, 50.0),
            r(500.0, 200.0, 400.0, 50.0),
        ];
        let edges = [(0, 1), (0, 2), (0, 1)];
        let ids = ids(&["a", "b", "c"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1, 2],
            ids: &ids,
        };
        // leaving point x (REFK points in): e0 = 30, e1 = 20, e2 = 20; e1 and e2 tie, and the far
        // end x (300 against 200) decides
        let long = |q: f64, last: f64| {
            let mut pts: Vec<Pt> = (0..30).map(|_| p(50.0, 0.0)).collect();
            pts[REFK] = p(q, 0.0);
            pts[29] = p(last, 100.0);
            vec![cd(false, Side::T, Side::T, &pts)]
        };
        let cands = vec![long(30.0, 0.0), long(20.0, 300.0), long(20.0, 200.0)];
        let po = rt.spread_ports(&cands, &[0, 0, 0]);
        // order: e2, e1, e0
        assert_eq!(
            offs(&po, &[(2, End::A), (1, End::A), (0, End::A)]),
            [-26.0, 0.0, 26.0]
        );
    }

    #[test]
    fn top_ties_keep_input_order_but_left_right_ties_break_by_id() {
        let pos = [
            r(0.0, 0.0, 100.0, 100.0),
            r(500.0, 0.0, 100.0, 100.0),
            r(900.0, 0.0, 100.0, 100.0),
        ];
        let edges = [(0, 1), (0, 2)];
        let ids = ids(&["b", "a"]); // edge 1 has the smaller id
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let same = |sd: Side| {
            let c = || cd(false, sd, sd, &[p(0.0, 0.0), p(0.0, 0.0)]);
            vec![vec![c()], vec![c()]]
        };
        // identical routes on the top side: input order, edge 0 first
        let po = rt.spread_ports(&same(Side::T), &[0, 0]);
        assert_eq!(offs(&po, &[(0, End::A), (1, End::A)]), [-13.0, 13.0]);
        // on the left side the ids decide: edge 1 ("a") first
        let po = rt.spread_ports(&same(Side::L), &[0, 0]);
        assert_eq!(offs(&po, &[(1, End::A), (0, End::A)]), [-13.0, 13.0]);
    }

    #[test]
    fn left_right_sides_order_by_y_and_ends_of_one_edge_by_a_then_b() {
        let pos = [r(0.0, 0.0, 100.0, 100.0), r(500.0, 0.0, 100.0, 100.0)];
        // a loop over one brick's left side (never routed in practice, but a clean tie of ends)
        let edges = [(0, 0), (0, 1)];
        let ids = ids(&["a", "b"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let flat = |y: f64| cd(false, Side::L, Side::L, &[p(0.0, y), p(0.0, y)]);
        // edge 1 leaves higher up (y 5) than edge 0 (y 50): it comes first
        let cands = vec![vec![flat(50.0)], vec![flat(5.0)]];
        let po = rt.spread_ports(&cands, &[0, 0]);
        // three ends on brick 0's left side (A and B of edge 0, A of edge 1); edge 1 first, then
        // edge 0's ends A before B on their tie; 80 usable, 3 ends -> step 26
        assert_eq!(
            offs(&po, &[(1, End::A), (0, End::A), (0, End::B)]),
            [-26.0, 0.0, 26.0]
        );
        assert_eq!(po[&(1, End::B)].off, 0.0);
    }

    #[test]
    fn the_leaving_point_is_refk_points_in_and_clamped_on_short_routes() {
        let pos = [
            r(0.0, 0.0, 100.0, 50.0),
            r(0.0, 200.0, 400.0, 50.0),
            r(500.0, 200.0, 400.0, 50.0),
        ];
        let edges = [(0, 1), (0, 2)];
        let ids = ids(&["a", "b"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        // 30 points: the A end looks at index REFK, not at the first or last point
        let long = |q: f64, last: f64| {
            let mut pts: Vec<Pt> = (0..30).map(|_| p(50.0, 0.0)).collect();
            pts[REFK] = p(q, 0.0);
            pts[29] = p(last, 100.0);
            vec![cd(false, Side::T, Side::T, &pts)]
        };
        // e0: q 100, last 0; e1: q 50, last 90 -> by q e1 comes first (by last it would be e0)
        let po = rt.spread_ports(&[long(100.0, 0.0), long(50.0, 90.0)], &[0, 0]);
        assert!(po[&(1, End::A)].off < po[&(0, End::A)].off);
        // the B end looks REFK from the end: index 29 - REFK, with the first point as far end
        let b_end = |q: f64| {
            let mut pts: Vec<Pt> = (0..30).map(|_| p(0.0, 0.0)).collect();
            pts[29 - REFK] = p(q, 0.0);
            vec![cd(false, Side::T, Side::T, &pts)]
        };
        // both B ends land on brick 1's top; q 300 vs 7 decides who is left of whom
        let edges = [(0, 1), (0, 1)];
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let po = rt.spread_ports(&[b_end(300.0), b_end(7.0)], &[0, 0]);
        assert!(po[&(1, End::B)].off < po[&(0, End::B)].off);
        // a two-point route clamps k to 1: A uses the last point, B the first
        let cands = vec![
            seg((60.0, 0.0), (10.0, 90.0)),
            seg((40.0, 0.0), (90.0, 90.0)),
        ];
        let po = rt.spread_ports(&cands, &[0, 0]);
        // A ends sort by their q = pts[1].x: e0 (10) before e1 (90)
        assert!(po[&(0, End::A)].off < po[&(1, End::A)].off);
    }

    #[test]
    fn different_sides_and_bricks_are_spread_separately() {
        let pos = [r(0.0, 0.0, 100.0, 100.0), r(500.0, 0.0, 100.0, 100.0)];
        let edges = [(0, 1), (0, 1)];
        let ids = ids(&["a", "b"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let cands = vec![
            vec![cd(
                false,
                Side::R,
                Side::L,
                &[p(100.0, 50.0), p(500.0, 50.0)],
            )],
            vec![cd(
                false,
                Side::B,
                Side::T,
                &[p(50.0, 100.0), p(550.0, 100.0)],
            )],
        ];
        let po = rt.spread_ports(&cands, &[0, 0]);
        assert!(po.values().all(|q| q.off == 0.0));
        assert_eq!(po[&(1, End::A)].side, Side::B);
        assert_eq!(po[&(1, End::B)].side, Side::T);
    }

    #[test]
    fn spreading_uses_the_chosen_candidate_of_each_edge() {
        let pos = [r(0.0, 0.0, 100.0, 100.0), r(500.0, 0.0, 100.0, 100.0)];
        let edges = [(0, 1)];
        let ids = ids(&["a"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let cands = vec![vec![
            cd(false, Side::R, Side::L, &[p(100.0, 50.0), p(500.0, 50.0)]),
            cd(false, Side::B, Side::T, &[p(50.0, 100.0), p(550.0, 100.0)]),
        ]];
        assert_eq!(rt.spread_ports(&cands, &[1])[&(0, End::A)].side, Side::B);
        assert_eq!(rt.spread_ports(&cands, &[0])[&(0, End::A)].side, Side::R);
    }

    /// Boxes 0 (top) and 1 (bottom) with one direct bottom-to-top candidate that has stale points.
    fn column_case(blocker: Option<Rect>) -> (Vec<Rect>, Vec<Vec<Cand>>) {
        let mut pos = vec![r(0.0, 0.0, 100.0, 50.0), r(0.0, 300.0, 100.0, 50.0)];
        pos.extend(blocker);
        let stale = [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)];
        let direct = cd(false, Side::B, Side::T, &stale);
        let old = cd(true, Side::B, Side::T, &stale);
        (pos, vec![vec![old, direct]])
    }

    #[test]
    fn settle_resamples_a_free_direct_route_with_the_spread_ports() {
        let (pos, mut cands) = column_case(None);
        let edges = [(0, 1)];
        let ids = ids(&["e"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let mut choice = vec![1];
        rt.settle(&mut cands, &mut choice);
        assert_eq!(choice, [1]);
        let c = &cands[0][1];
        assert!(c.pts.len() > 3, "re-sampled at step 4");
        assert_eq!(c.pts[0], bz_port(&pos[0], Side::B, 0.0));
        assert_eq!(*c.pts.last().unwrap(), bz_port(&pos[1], Side::T, 0.0));
        assert_eq!(c.bb, bbox(&c.pts));
        assert_eq!(c.cp, coarse(&c.pts));
        // the old route is left alone
        assert_eq!(cands[0][0].pts.len(), 3);
    }

    #[test]
    fn settle_resets_a_route_that_hits_a_box_to_the_layered_one() {
        let (pos, mut cands) = column_case(Some(r(0.0, 150.0, 100.0, 50.0)));
        let edges = [(0, 1)];
        let ids = ids(&["e"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let mut choice = vec![1];
        rt.settle(&mut cands, &mut choice);
        assert_eq!(choice, [0]);
        // the reset candidate's points are left as they were
        assert_eq!(cands[0][1].pts.len(), 3);
    }

    #[test]
    fn settle_leaves_old_choices_and_unrouted_edges_alone() {
        let (pos, mut cands) = column_case(Some(r(0.0, 150.0, 100.0, 50.0)));
        cands.push(vec![]);
        let edges = [(0, 1), (1, 0)];
        let ids = ids(&["e", "f"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let mut choice = vec![0, 0];
        rt.settle(&mut cands, &mut choice);
        assert_eq!(choice, [0, 0]);
        assert_eq!(cands[0][0].pts.len(), 3);
    }

    #[test]
    fn settle_respreads_after_a_reset_so_the_other_edge_recentres() {
        // Edges 0 and 1 both leave brick 0's bottom for brick 1's top and share both sides, so
        // spreading puts them at -13 / +13. A blocker that only the +13 route meets resets edge 1;
        // edge 0 is then alone on both sides and is re-sampled from offset 0 in the next round.
        let pos = vec![
            r(0.0, 0.0, 100.0, 50.0),
            r(0.0, 300.0, 100.0, 50.0),
            r(60.0, 150.0, 20.0, 50.0),
        ];
        let edges = [(0, 1), (0, 1)];
        let ids = ids(&["a", "b"]);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let stale = |x: f64| [p(x, 50.0), p(x, 300.0)];
        // old routes leave by the left sides, so a reset edge gives up its slots on the bottom/top
        let mk = |x: f64| {
            vec![
                cd(true, Side::L, Side::L, &stale(x)),
                cd(false, Side::B, Side::T, &stale(x)),
            ]
        };
        let mut cands = vec![mk(40.0), mk(60.0)];
        let mut choice = vec![1, 1];
        rt.settle(&mut cands, &mut choice);
        assert_eq!(choice, [1, 0]);
        assert_eq!(cands[0][1].pts[0], bz_port(&pos[0], Side::B, 0.0));
    }

    #[test]
    fn top_ties_keep_input_order_and_are_reproducible() {
        let ids = vec!["b".to_string(), "a".to_string()];
        let rt = Router {
            pos: &[],
            edges: &[],
            eids: &[],
            ids: &ids,
        };
        let q = Pt { x: 5.0, y: 0.0 };
        let run = || {
            // Equal keys, input order edge 0 (id "b") then edge 1 (id "a").
            let mut list: Vec<Pl> = (0..2)
                .map(|e| Pl {
                    key: (e, End::A),
                    q,
                    far: q,
                })
                .collect();
            rt.sort_side(&mut list, Side::T);
            list.iter().map(|p| p.key.0).collect::<Vec<_>>()
        };
        // No id tie-break on top: the id order ("a" first) is not applied.
        assert_eq!(run(), [0, 1]);
        assert_eq!(run(), run());
    }
}
