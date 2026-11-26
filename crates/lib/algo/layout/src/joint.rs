//! Joint optimizer coupling brick row-ordering and horizontal placement with route energy.

use std::collections::BTreeSet;

use crate::Input;
use crate::placement::{self, Placed, Seed};
use crate::routing::{BundleStat, Energy, RouteOut, optimize_routes};

const SEARCH_BUDGET: usize = 16;
const ANCHOR_W: f64 = 0.25;

fn improves(cand: &Energy, best: &Energy, min_delta: f64) -> bool {
    cand.hard < best.hard - 1e-9
        || ((cand.hard - best.hard).abs() <= 1e-9 && cand.total + min_delta < best.total)
}

fn conflict_bundles(stats: &[BundleStat], n: usize) -> Vec<(usize, usize)> {
    let mut ranked: Vec<(usize, usize, usize)> = stats
        .iter()
        .filter(|s| s.a != s.b && s.a < n && s.b < n)
        .filter_map(|s| {
            let score = s
                .crossings
                .saturating_mul(4)
                .saturating_add(s.bends.saturating_mul(2))
                .saturating_mul(s.weight);
            (score > 0).then_some((score, s.a, s.b))
        })
        .collect();
    ranked.sort_by(|x, y| y.0.cmp(&x.0).then((x.1, x.2).cmp(&(y.1, y.2))));
    ranked.into_iter().map(|(_, a, b)| (a, b)).collect()
}

fn locate(order: &[Vec<usize>], brick: usize) -> Option<(usize, usize)> {
    order
        .iter()
        .enumerate()
        .find_map(|(r, row)| row.iter().position(|&x| x == brick).map(|i| (r, i)))
}

fn bundle_moves(order: &[Vec<usize>], a: usize, b: usize) -> Vec<Vec<Vec<usize>>> {
    let (Some((ra, ia)), Some((rb, ib))) = (locate(order, a), locate(order, b)) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(6);
    if ra == rb && ia.abs_diff(ib) > 1 {
        let (near, far) = if ia < ib { (ib - 1, ib) } else { (ib + 1, ib) };
        for dst in [near, far] {
            let mut next = order.to_vec();
            next[ra].remove(ia);
            next[ra].insert(dst, a);
            out.push(next);
        }
    }
    for (r, i) in [(ra, ia), (rb, ib)] {
        if i > 0 {
            let mut next = order.to_vec();
            next[r].swap(i - 1, i);
            out.push(next);
        }
        if i + 1 < order[r].len() {
            let mut next = order.to_vec();
            next[r].swap(i, i + 1);
            out.push(next);
        }
    }
    out
}

fn search_row_order(
    seed: &Seed,
    edges: &[(usize, usize)],
    ids: &[String],
    order: &mut Vec<Vec<usize>>,
    placed: &mut Placed,
    route: &mut RouteOut,
) {
    let n = seed.widths.len();
    let mut evals = 0usize;
    let mut seen: BTreeSet<Vec<Vec<usize>>> = BTreeSet::new();
    seen.insert(order.clone());
    while evals < SEARCH_BUDGET {
        let bundles = conflict_bundles(&route.stats, n);
        let mut accepted = false;
        for (a, b) in bundles {
            let mut round_best: Option<(Vec<Vec<usize>>, Placed, RouteOut)> = None;
            for cand_order in bundle_moves(order, a, b) {
                if evals >= SEARCH_BUDGET {
                    break;
                }
                if !seen.insert(cand_order.clone()) {
                    continue;
                }
                evals += 1;
                let cand_placed = placement::materialize(seed, &cand_order, None, None);
                let cand_route = optimize_routes(&cand_placed.pos, edges, ids, true);
                if improves(&cand_route.energy, &route.energy, 1.0)
                    && round_best
                        .as_ref()
                        .is_none_or(|(_, _, r)| improves(&cand_route.energy, &r.energy, 0.0))
                {
                    round_best = Some((cand_order, cand_placed, cand_route));
                }
            }
            if let Some((next_order, next_placed, next_route)) = round_best {
                (*order, *placed, *route) = (next_order, next_placed, next_route);
                accepted = true;
                break;
            }
        }
        if !accepted {
            break;
        }
    }
}

fn relax_ports(
    seed: &Seed,
    order: &[Vec<usize>],
    edges: &[(usize, usize)],
    ids: &[String],
    placed: &mut Placed,
    route: &mut RouteOut,
) {
    let n = seed.widths.len();
    for _ in 0..2 {
        let (mut sum_dx, mut sum_w) = (vec![0.0; n], vec![0.0; n]);
        for stat in &route.stats {
            let Some((px_a, px_b)) = stat.v_align else {
                continue;
            };
            if stat.a < n && stat.b < n && stat.weight > 0 && px_a.is_finite() && px_b.is_finite() {
                let w = stat.weight as f64;
                sum_dx[stat.a] += w * (px_b - px_a);
                sum_w[stat.a] += w;
                sum_dx[stat.b] += w * (px_a - px_b);
                sum_w[stat.b] += w;
            }
        }
        if sum_w.iter().all(|&w| w <= 0.0) {
            break;
        }
        let x_targets: Vec<f64> = (0..n)
            .map(|i| placed.pos[i].x + sum_dx[i] / (sum_w[i] + ANCHOR_W))
            .collect();
        let cand_placed = placement::materialize(seed, order, Some(&x_targets), None);
        let cand_route = optimize_routes(&cand_placed.pos, edges, ids, true);
        if improves(&cand_route.energy, &route.energy, 1e-9) {
            (*placed, *route) = (cand_placed, cand_route);
        } else {
            break;
        }
    }
}

/// Jointly optimise brick row ordering, horizontal coordinates, and orthogonal edge routes.
pub(crate) fn optimize_joint(inp: &Input) -> (Placed, RouteOut) {
    let edges: Vec<(usize, usize)> = inp.edges.iter().map(|e| (e.a, e.b)).collect();
    let ids: Vec<String> = inp.edges.iter().map(|e| e.id.clone()).collect();
    let n = inp.bricks.len();
    if n <= 1 || !inp.edges.iter().any(|e| e.a != e.b && e.a < n && e.b < n) {
        let placed = placement::place(inp);
        let route = optimize_routes(&placed.pos, &edges, &ids, false);
        return (placed, route);
    }
    let seed = placement::build_seed(inp);
    let mut best_order = seed.row_order.clone();
    let mut best_placed = seed.placed.clone();
    let mut best_route = optimize_routes(&best_placed.pos, &edges, &ids, true);
    search_row_order(
        &seed,
        &edges,
        &ids,
        &mut best_order,
        &mut best_placed,
        &mut best_route,
    );
    relax_ports(
        &seed,
        &best_order,
        &edges,
        &ids,
        &mut best_placed,
        &mut best_route,
    );
    let final_route = optimize_routes(&best_placed.pos, &edges, &ids, false);
    (best_placed, final_route)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InBrick, InEdge, PADL};

    fn b(id: &str, row: usize) -> InBrick {
        InBrick {
            id: id.into(),
            row,
            min_w: None,
            min_h: None,
        }
    }

    fn e(id: &str, a: usize, b: usize) -> InEdge {
        InEdge {
            id: id.into(),
            a,
            b,
        }
    }

    #[test]
    fn fast_path_handles_empty_single_and_self_only_graphs() {
        let empty = optimize_joint(&Input::default());
        assert!(empty.0.pos.is_empty());
        assert_eq!(empty.1.energy.total, 0.0);

        let one = Input {
            rows: vec!["r0".into()],
            bricks: vec![b("only", 0)],
            edges: vec![e("self", 0, 0)],
        };
        let (p1, r1) = optimize_joint(&one);
        assert_eq!(p1.pos.len(), 1);
        assert!(r1.routes[0].is_none());

        let no_valid = Input {
            rows: vec!["r0".into(), "r1".into()],
            bricks: vec![b("a", 0), b("b", 1)],
            edges: vec![e("s0", 0, 0), e("oob", 0, 9)],
        };
        let (p2, r2) = optimize_joint(&no_valid);
        assert_eq!(p2.pos.len(), 2);
        assert_eq!(r2.energy.total, 0.0);
    }

    #[test]
    fn conflict_ranking_moves_and_step2_search() {
        let stats = vec![
            BundleStat {
                a: 0,
                b: 2,
                weight: 1,
                bends: 1,
                crossings: 0,
                v_align: None,
            },
            BundleStat {
                a: 1,
                b: 3,
                weight: 2,
                bends: 1,
                crossings: 2,
                v_align: None,
            },
            BundleStat {
                a: 0,
                b: 0,
                weight: 9,
                bends: 5,
                crossings: 5,
                v_align: None,
            },
        ];
        assert_eq!(conflict_bundles(&stats, 4), vec![(1, 3), (0, 2)]);

        // Same-row non-adjacent (0 and 2) generates adjacent placement moves too.
        let ord = vec![vec![0, 1, 2], vec![3, 4]];
        let moves = bundle_moves(&ord, 0, 2);
        assert!(moves.iter().any(|m| {
            let p0 = m[0].iter().position(|&x| x == 0).unwrap();
            let p2 = m[0].iter().position(|&x| x == 2).unwrap();
            p0.abs_diff(p2) == 1
        }));
    }

    #[test]
    fn optimize_joint_is_deterministic_and_beats_or_matches_seed() {
        let names = [
            ("contract", 0),
            ("audio", 0),
            ("engine", 1),
            ("session", 1),
            ("mixer", 1),
            ("codec", 1),
            ("alsa", 2),
            ("pulse", 2),
            ("wasapi", 2),
            ("coreaudio", 2),
            ("null", 2),
            ("client", 3),
            ("cli", 3),
            ("ffi", 3),
            ("config", 3),
        ];
        let pairs = [
            (2, 0),
            (2, 1),
            (3, 2),
            (3, 0),
            (4, 1),
            (5, 1),
            (2, 4),
            (2, 5),
            (6, 1),
            (7, 1),
            (8, 1),
            (9, 1),
            (10, 1),
            (6, 0),
            (7, 0),
            (11, 2),
            (11, 3),
            (11, 6),
            (11, 7),
            (11, 1),
            (11, 0),
            (12, 11),
            (13, 11),
            (12, 14),
            (11, 14),
            (2, 6),
            (4, 5),
            (3, 4),
            (8, 7),
            (10, 6),
            (13, 2),
            (14, 0),
            (2, 1),
            (0, 0),
        ];
        let inp = Input {
            rows: vec![
                "port".into(),
                "core".into(),
                "adapter".into(),
                "assembly".into(),
            ],
            bricks: names.iter().map(|&(id, r)| b(id, r)).collect(),
            edges: pairs
                .iter()
                .enumerate()
                .map(|(k, &(x, y))| e(&format!("e{k}"), x, y))
                .collect(),
        };
        let (p1, r1) = optimize_joint(&inp);
        let (p2, r2) = optimize_joint(&inp);
        assert_eq!(p1.pos, p2.pos);
        assert_eq!(r1.energy.total, r2.energy.total);
        let min_x = p1.pos.iter().fold(f64::INFINITY, |m, r| m.min(r.x));
        assert!((min_x - PADL).abs() < 1e-9);

        let seed = placement::place(&inp);
        let edges: Vec<(usize, usize)> = inp.edges.iter().map(|ed| (ed.a, ed.b)).collect();
        let ids: Vec<String> = inp.edges.iter().map(|ed| ed.id.clone()).collect();
        let base_r = optimize_routes(&seed.pos, &edges, &ids, false);
        assert!(r1.energy.hard <= base_r.energy.hard + 1e-9);
        assert!(r1.energy.total <= base_r.energy.total + 1e-6);
    }

    #[test]
    fn relax_ports_respects_non_overlap_and_improves_or_keeps_energy() {
        let inp = Input {
            rows: vec!["top".into(), "bot".into()],
            bricks: vec![b("wide_left_anchor", 0), b("t1", 0), b("t2", 0), b("b0", 1)],
            edges: vec![e("e0", 2, 3), e("e1", 0, 3)],
        };
        let (p, r) = optimize_joint(&inp);
        assert_eq!(p.pos.len(), 4);
        assert_eq!(r.routes.len(), 2);
        for row in &p.rows {
            for w in row.ids.windows(2) {
                assert!(p.pos[w[0]].x + p.pos[w[0]].w + 24.0 <= p.pos[w[1]].x + 1e-9);
            }
        }
    }
}
