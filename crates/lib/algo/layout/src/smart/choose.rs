//! Choosing one candidate per edge: the cost function and the greedy two-pass search.

use super::super::geometry::{bb_apart, poly_cross, poly_near};
use super::*;

/// Side capacity: how many ports of one side fit, `PORT` apart with `PAD` at both ends (at least 1).
pub(super) fn cap(n: &Rect, sd: Side) -> usize {
    (((n.side_len(sd) - 2.0 * PAD) / PORT).floor() as i64 + 1).max(1) as usize
}

/// True when edges with ends `f` and `g` meet at a brick.
fn share_end(f: (usize, usize), g: (usize, usize)) -> bool {
    f.0 == g.0 || f.0 == g.1 || f.1 == g.0 || f.1 == g.1
}

/// Ports in use per (brick, side) by the candidates chosen so far.
#[derive(Default)]
struct Used(HashMap<(usize, Side), usize>);

impl Used {
    fn get(&self, brick: usize, sd: Side) -> usize {
        self.0.get(&(brick, sd)).copied().unwrap_or(0)
    }

    /// Count (`add`) or uncount candidate `c` of an edge with ends `ends` on both its sides.
    fn take(&mut self, ends: (usize, usize), c: &Cand, add: bool) {
        for k in [(ends.0, c.sa), (ends.1, c.sb)] {
            let v = self.0.entry(k).or_insert(0);
            *v = if add { *v + 1 } else { v.saturating_sub(1) };
        }
    }
}

impl Router<'_> {
    /// Cost of giving edge `e` candidate `c` given the others' current `choice` (`None` = not
    /// chosen yet, skipped; `e`'s own entry is skipped too): box hits x 1000, plus crossings with
    /// the chosen candidates x 30, plus closeness to those of edges that share no brick x 0.6,
    /// plus the candidate's penalty and 0.02 per unit of length. Chosen candidates whose bounding
    /// box is more than 8 away are ignored.
    pub(super) fn cost(
        &self,
        e: usize,
        c: &Cand,
        choice: &[Option<usize>],
        cands: &[Vec<Cand>],
    ) -> f64 {
        let (mut x, mut near) = (0.0, 0.0);
        for &o in self.eids {
            let Some(ci) = choice[o] else { continue };
            if o == e {
                continue;
            }
            let oc = &cands[o][ci];
            if bb_apart(&c.bb, &oc.bb, 8.0) {
                continue;
            }
            x += poly_cross(&c.cp, &oc.cp) as f64;
            if !share_end(self.edges[o], self.edges[e]) {
                near += poly_near(&c.cp, &oc.pts, 8.0);
            }
        }
        c.boxh as f64 * 1000.0 + x * 30.0 + near * 0.6 + c.pen + c.len * 0.02
    }

    /// Routed edges, shortest first: by their shortest candidate, ties by edge id.
    pub(super) fn order(&self, cands: &[Vec<Cand>]) -> Vec<usize> {
        let min_len = |e: usize| cands[e].iter().map(|c| c.len).fold(f64::INFINITY, f64::min);
        let mut order = self.eids.to_vec();
        order.sort_by(|&p, &q| {
            min_len(p)
                .total_cmp(&min_len(q))
                .then(self.ids[p].cmp(&self.ids[q]))
        });
        order
    }

    /// True when direct candidate `c` of edge `e` would exceed the capacity of a side it uses.
    fn over_cap(&self, e: usize, c: &Cand, used: &Used) -> bool {
        let (a, b) = self.edges[e];
        !c.old
            && (used.get(a, c.sa) >= cap(&self.pos[a], c.sa)
                || used.get(b, c.sb) >= cap(&self.pos[b], c.sb))
    }

    /// Cheapest candidate index of edge `e` that respects the side capacities (the old route is
    /// always allowed); the first one wins ties, and index 0 when nothing is finite.
    fn pick(&self, e: usize, cands: &[Vec<Cand>], choice: &[Option<usize>], used: &Used) -> usize {
        let (mut best, mut bc) = (0, f64::INFINITY);
        for (ci, c) in cands[e].iter().enumerate() {
            if self.over_cap(e, c, used) {
                continue;
            }
            let v = self.cost(e, c, choice, cands);
            if v < bc {
                bc = v;
                best = ci;
            }
        }
        best
    }

    /// Greedy choice: two passes over `order`; each edge releases its previous choice (and its
    /// port counts) and re-picks against everything chosen so far. Old routes count against the
    /// capacity of their sides like direct ones, they are just never refused. Returns one candidate index per
    /// edge (0 for edges that are not routed).
    pub(super) fn greedy(&self, cands: &[Vec<Cand>], order: &[usize]) -> Vec<usize> {
        let mut used = Used::default();
        let mut choice: Vec<Option<usize>> = vec![None; self.edges.len()];
        for _pass in 0..2 {
            for &e in order {
                if let Some(ci) = choice[e].take() {
                    used.take(self.edges[e], &cands[e][ci], false);
                }
                let best = self.pick(e, cands, &choice, &used);
                choice[e] = Some(best);
                used.take(self.edges[e], &cands[e][best], true);
            }
        }
        choice.iter().map(|c| c.unwrap_or(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smart::cand::Cand;

    fn p(x: f64, y: f64) -> Pt {
        Pt { x, y }
    }

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    /// A candidate over `pts` with all derived fields filled, as `candidates` does.
    fn cd(old: bool, sa: Side, sb: Side, pen: f64, pts: &[Pt]) -> Cand {
        Cand::new(old, sa, sb, pen, pts.to_vec(), 0)
    }

    fn plain(pen: f64) -> Cand {
        cd(false, Side::T, Side::T, pen, &[p(0.0, 0.0)])
    }

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("e{i}")).collect()
    }

    /// Router parts: `n` wide boxes, so capacity is never the limit.
    fn wide(n: usize) -> Vec<Rect> {
        (0..n)
            .map(|i| r(i as f64 * 500.0, 0.0, 400.0, 50.0))
            .collect()
    }

    const HORIZONTAL: [(f64, f64); 2] = [(0.0, 5.0), (20.0, 5.0)];
    const VERTICAL: [(f64, f64); 2] = [(10.0, 0.0), (10.0, 10.0)];

    fn line(a: [(f64, f64); 2]) -> Vec<Pt> {
        a.iter().map(|&(x, y)| p(x, y)).collect()
    }

    #[test]
    fn cap_per_side() {
        let n = r(0.0, 0.0, 100.0, 40.0);
        // (100 - 20) / 12 = 6.67 -> 6 + 1
        assert_eq!(cap(&n, Side::T), 7);
        // (40 - 20) / 12 = 1.67 -> 1 + 1
        assert_eq!(cap(&n, Side::L), 2);
        // exact fit and degenerate sides still hold one port
        assert_eq!(cap(&r(0.0, 0.0, 32.0, 32.0), Side::T), 2);
        assert_eq!(cap(&r(0.0, 0.0, 20.0, 20.0), Side::T), 1);
        assert_eq!(cap(&r(0.0, 0.0, 5.0, 5.0), Side::T), 1);
    }

    #[test]
    fn cost_of_a_lone_candidate_is_hits_penalty_and_length() {
        let pos = wide(2);
        let edges = [(0, 1)];
        let ids = ids(1);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let mut c = cd(false, Side::T, Side::T, 7.0, &[p(0.0, 0.0), p(100.0, 0.0)]);
        c.boxh = 2;
        let v = rt.cost(0, &c, &[None], &[vec![]]);
        assert_eq!(v, 2.0 * 1000.0 + 7.0 + 100.0 * 0.02);
    }

    /// Edges 0 and 1 (not sharing a brick) with `other` chosen for edge 1.
    fn cost_against(
        other: Cand,
        edges: [(usize, usize); 2],
        own: Option<usize>,
        chosen: bool,
    ) -> f64 {
        let pos = wide(4);
        let ids = ids(2);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let mine = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL));
        let cands = vec![vec![plain(0.0)], vec![other]];
        let choice = [own, chosen.then_some(0)];
        rt.cost(0, &mine, &choice, &cands)
    }

    #[test]
    fn cost_counts_crossings_and_nearby_points_of_chosen_edges() {
        let mine = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL));
        let other = || cd(false, Side::T, Side::T, 0.0, &line(VERTICAL));
        let x = poly_cross(&mine.cp, &other().cp) as f64;
        assert_eq!(x, 1.0);
        let near = poly_near(&mine.cp, &other().pts, 8.0);
        assert_eq!(near, 0.0);
        let base = mine.len * 0.02;
        // crossing, no shared brick
        assert_eq!(
            cost_against(other(), [(0, 1), (2, 3)], None, true),
            base + 30.0
        );
        // crossing at a shared brick: still a crossing
        assert_eq!(
            cost_against(other(), [(0, 1), (1, 2)], None, true),
            base + 30.0
        );
        // not chosen yet: ignored
        assert_eq!(cost_against(other(), [(0, 1), (2, 3)], None, false), base);
    }

    #[test]
    fn cost_adds_closeness_only_for_edges_sharing_no_brick() {
        // parallel lines 3 apart: no crossing, every point of mine is near theirs
        let near_line = cd(false, Side::T, Side::T, 0.0, &[p(0.0, 8.0), p(20.0, 8.0)]);
        let mine = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL));
        let near = poly_near(&mine.cp, &near_line.pts, 8.0);
        assert_eq!(near, 28.0);
        let base = mine.len * 0.02;
        let apart = cost_against(
            cd(false, Side::T, Side::T, 0.0, &[p(0.0, 8.0), p(20.0, 8.0)]),
            [(0, 1), (2, 3)],
            None,
            true,
        );
        assert_eq!(apart, base + near * 0.6);
        let shared = cost_against(near_line, [(0, 1), (0, 2)], None, true);
        assert_eq!(shared, base);
    }

    #[test]
    fn cost_skips_edges_whose_boxes_are_more_than_eight_apart() {
        let far = cd(
            false,
            Side::T,
            Side::T,
            0.0,
            &[p(10.0, 100.0), p(10.0, 110.0)],
        );
        let base = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL)).len * 0.02;
        assert_eq!(cost_against(far, [(0, 1), (2, 3)], None, true), base);
    }

    #[test]
    fn cost_ignores_the_edges_own_choice() {
        let own = cd(false, Side::T, Side::T, 0.0, &line(VERTICAL));
        let base = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL)).len * 0.02;
        let pos = wide(4);
        let ids = ids(2);
        let edges = [(0, 1), (2, 3)];
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let mine = cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL));
        let cands = vec![vec![own], vec![plain(0.0)]];
        // edge 0's own (crossing) choice must not count against it
        assert_eq!(rt.cost(0, &mine, &[Some(0), None], &cands), base);
    }

    #[test]
    fn order_is_shortest_first_then_by_id() {
        let pos = wide(2);
        let edges = [(0, 1); 4];
        let ids: Vec<String> = ["b", "a", "c", "z"].map(String::from).to_vec();
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1, 2, 3],
            ids: &ids,
        };
        let sized = |lens: &[f64]| {
            lens.iter()
                .map(|&l| {
                    let mut c = plain(0.0);
                    c.len = l;
                    c
                })
                .collect::<Vec<_>>()
        };
        // edge 0: min 5 (of 9, 5); edge 1: 5; edge 2: 1; edge 3: no candidates (infinite)
        let cands = vec![sized(&[9.0, 5.0]), sized(&[5.0]), sized(&[1.0]), vec![]];
        // 2 (len 1), then the tie of 0 and 1 by id ("a" is edge 1), then the infinite one
        assert_eq!(rt.order(&cands), vec![2, 1, 0, 3]);
    }

    #[test]
    fn order_only_lists_routed_edges() {
        let pos = wide(2);
        let edges = [(0, 1); 3];
        let ids = ids(3);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 2],
            ids: &ids,
        };
        let cands = vec![vec![plain(0.0)], vec![], vec![plain(0.0)]];
        assert_eq!(rt.order(&cands), vec![0, 2]);
    }

    #[test]
    fn greedy_picks_the_cheapest_candidate_first_on_ties() {
        let pos = wide(2);
        let edges = [(0, 1), (0, 1)];
        let ids = ids(2);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let cands = vec![
            vec![plain(12.0), plain(3.0), plain(3.0), plain(9.0)],
            vec![],
        ];
        // lowest cost at index 1, the equal index 2 does not replace it; edge 1 is not routed
        assert_eq!(rt.greedy(&cands, &[0]), vec![1, 0]);
        let cands = vec![vec![plain(1.0), plain(1.0)], vec![]];
        assert_eq!(rt.greedy(&cands, &[0]), vec![0, 0]);
    }

    #[test]
    fn greedy_avoids_boxes_over_penalties() {
        let pos = wide(2);
        let edges = [(0, 1)];
        let ids = ids(1);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0],
            ids: &ids,
        };
        let mut hit = plain(0.0);
        hit.boxh = 1;
        assert_eq!(rt.greedy(&[vec![hit, plain(45.0)]], &[0]), vec![1]);
    }

    /// Edges 0 and 1 both start on brick 0, whose top side holds only one port.
    fn narrow_start() -> (Vec<Rect>, [(usize, usize); 2]) {
        let mut pos = wide(3);
        pos[0] = r(0.0, 0.0, 20.0, 50.0); // cap(T) = 1
        (pos, [(0, 1), (0, 2)])
    }

    fn old_and_direct() -> Vec<Cand> {
        vec![
            cd(true, Side::L, Side::L, 45.0, &[p(0.0, 0.0)]),
            cd(false, Side::T, Side::T, 0.0, &[p(0.0, 0.0)]),
        ]
    }

    #[test]
    fn greedy_respects_side_capacity_but_always_allows_the_old_route() {
        let (pos, edges) = narrow_start();
        let ids = ids(2);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let cands = vec![old_and_direct(), old_and_direct()];
        // the first edge takes the only port; the second falls back to its old route
        assert_eq!(rt.greedy(&cands, &[0, 1]), vec![1, 0]);
        assert_eq!(rt.greedy(&cands, &[1, 0]), vec![0, 1]);
        // with room for both ends (wide boxes) both go direct
        let pos = wide(3);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        assert_eq!(rt.greedy(&cands, &[0, 1]), vec![1, 1]);
    }

    #[test]
    fn greedy_capacity_counts_the_far_end_too() {
        let mut pos = wide(3);
        pos[2] = r(1000.0, 0.0, 20.0, 50.0); // cap(T) = 1 at the shared target brick
        let edges = [(0, 1), (0, 2), (1, 2)];
        let ids = ids(3);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[1, 2],
            ids: &ids,
        };
        let cands = vec![vec![], old_and_direct(), old_and_direct()];
        assert_eq!(rt.greedy(&cands, &[1, 2]), vec![0, 1, 0]);
    }

    #[test]
    fn greedy_second_pass_reconsiders_against_later_choices() {
        let pos = wide(4);
        let edges = [(0, 1), (2, 3)];
        let ids = ids(2);
        let rt = Router {
            pos: &pos,
            edges: &edges,
            eids: &[0, 1],
            ids: &ids,
        };
        let e0 = vec![
            cd(
                false,
                Side::T,
                Side::T,
                10.0,
                &[p(0.0, 100.0), p(20.0, 100.0)],
            ),
            cd(false, Side::T, Side::T, 0.0, &line(HORIZONTAL)),
        ];
        let e1 = vec![
            cd(
                false,
                Side::T,
                Side::T,
                40.0,
                &[p(10.0, 40.0), p(10.0, 50.0)],
            ),
            cd(false, Side::T, Side::T, 0.0, &line(VERTICAL)),
        ];
        // pass 1: edge 0 takes its free pen-0 route; edge 1 then takes the crossing one (30 < 40).
        // pass 2: edge 0 now sees the crossing and moves to pen 10; edge 1 stays.
        assert_eq!(rt.greedy(&[e0, e1], &[0, 1]), vec![0, 1]);
    }
}
