//! Row grouping, crossing minimisation, and isotonic (`pava`) placement.

use std::{array::from_fn, collections::BTreeMap, iter::repeat_n};

use crate::{GAPY, GAPY_FREE, H, Input, PAD, PADL, PADR, PORT, Rect, RowOut};

const GAP: f64 = 24.0;
const SAME_W: f64 = 0.5;

/// Output of row grouping, ordering, and box placement.
#[derive(Debug, Clone, Default)]
pub(crate) struct Placed {
    pub(crate) w: f64,
    pub(crate) h: f64,
    pub(crate) px0: f64,
    pub(crate) pos: Vec<Rect>,
    pub(crate) rows: Vec<RowOut>,
    pub(crate) cross: f64,
    pub(crate) cross0: f64,
}

/// Precomputed row metadata and brick adjacency for joint-search materialization.
#[derive(Debug, Clone, Default)]
pub(crate) struct Seed {
    pub(crate) placed: Placed,
    pub(crate) row_order: Vec<Vec<usize>>,
    pub(crate) widths: Vec<f64>,
    pub(crate) heights: Vec<f64>,
    pub(crate) row_y: Vec<f64>,
    pub(crate) row_h: Vec<f64>,
    pub(crate) row_labels: Vec<String>,
    adj_bricks: Adj,
}

/// Non-decreasing `y` minimising `sum w_i * (y_i - t_i)^2` (pool adjacent violators).
pub(crate) fn pava(t: &[f64], w: &[f64]) -> Vec<f64> {
    let mut blk: Vec<(f64, f64, f64, f64, usize)> = Vec::with_capacity(t.len());
    for (&ti, &wi) in t.iter().zip(w) {
        let w = wi.max(0.0);
        blk.push((ti, w, w * ti, ti, 1));
        while blk.len() >= 2 && blk[blk.len() - 2].0 > blk[blk.len() - 1].0 {
            let (b, a) = (blk.pop().unwrap(), blk.last_mut().unwrap());
            let (ws, tw, ts, n) = (a.1 + b.1, a.2 + b.2, a.3 + b.3, a.4 + b.4);
            let y = if ws > 0.0 { tw / ws } else { ts / n as f64 };
            *a = (y, ws, tw, ts, n);
        }
    }
    blk.into_iter().flat_map(|b| repeat_n(b.0, b.4)).collect()
}

type Adj = [Vec<Vec<(usize, f64)>>; 3];

fn crossings(layers: &[Vec<usize>], adj: &Adj, pos: &mut [(usize, usize)]) -> f64 {
    for (l, r) in layers.iter().enumerate() {
        r.iter().enumerate().for_each(|(i, &v)| pos[v] = (l, i));
    }
    let mut sum = 0.0;
    for (l, w) in layers.windows(2).enumerate() {
        let mut seen: Vec<(usize, f64)> = Vec::new();
        for &u in &w[0] {
            let b = seen.len();
            for &(v, wu) in adj[1][u].iter().filter(|&&(v, _)| pos[v].0 == l + 1) {
                seen.push((pos[v].1, wu));
                (0..b).for_each(|j| sum += wu * seen[j].1 * f64::from(seen[j].0 > pos[v].1));
            }
        }
    }
    for (u, nb) in adj[2].iter().enumerate() {
        for &(v, w) in nb.iter().filter(|&&(v, _)| u < v) {
            sum += SAME_W * w * pos[u].1.abs_diff(pos[v].1).saturating_sub(1) as f64;
        }
    }
    sum
}

fn minimise(init: &[Vec<usize>], adj: &Adj) -> (Vec<Vec<usize>>, f64, f64) {
    let (mut pos, nl) = (vec![(0usize, 0usize); adj[0].len()], init.len());
    let mut best_c = crossings(init, adj, &mut pos);
    if best_c == 0.0 || nl == 0 {
        return (init.to_vec(), best_c, best_c);
    }
    let (mut ly, cross0) = (init.to_vec(), best_c);
    for _ in 0..10 {
        let mut imp = false;
        for l in 0..nl {
            for v in ly[l].clone() {
                let mut best_p = ly[l].iter().position(|&x| x == v).unwrap();
                ly[l].remove(best_p);
                for p in 0..=ly[l].len() {
                    ly[l].insert(p, v);
                    let c = crossings(&ly, adj, &mut pos);
                    if c + 1e-9 < best_c {
                        (best_c, best_p, imp) = (c, p, true);
                    }
                    ly[l].remove(p);
                }
                ly[l].insert(best_p, v);
            }
        }
        if best_c == 0.0 || !imp {
            break;
        }
    }
    (ly, best_c, cross0)
}

fn row_offs(row: &[usize], w: &[f64], gap: f64) -> Vec<f64> {
    let mut off = vec![0.0; row.len()];
    (1..row.len()).for_each(|i| off[i] = off[i - 1] + 0.5 * (w[row[i - 1]] + w[row[i]]) + gap);
    off
}

fn pava_row(row: &[usize], cx: &mut [f64], off: &[f64], adj: &Adj, d0: usize, d1: usize) {
    let m = row.len();
    let (mut t, mut wt, mut sx, mut sw) = (vec![0.0; m], vec![0.0; m], 0.0, 0.0);
    for (i, &v) in row.iter().enumerate() {
        let (mut vx, mut vw) = (0.0, 0.0);
        for k in d0..d1 {
            for &(u, w) in &adj[k][v] {
                if k < 2 {
                    (vx, vw) = (vx + w * cx[u], vw + w);
                } else if let Some(iu) = row.iter().position(|&x| x == u) {
                    let x = cx[u] - off[iu] + off[i];
                    (vx, vw) = (vx + SAME_W * w * x, vw + SAME_W * w);
                }
            }
        }
        if vw > 0.0 {
            (t[i], wt[i], sx, sw) = (vx / vw, vw, sx + vx, sw + vw);
        }
    }
    if sw == 0.0 {
        return;
    }
    (0..m).for_each(|i| t[i] = (if wt[i] > 0.0 { t[i] } else { sx / sw }) - off[i]);
    let ys = pava(&t, &wt);
    (0..m).for_each(|i| cx[row[i]] = ys[i] + off[i]);
}

fn place_x(ly: &[Vec<usize>], w: &[f64], adj: &Adj, gaps: &[f64]) -> Vec<f64> {
    let (nl, mut cx, mut offs) = (ly.len(), vec![0.0; w.len()], Vec::with_capacity(ly.len()));
    for (l, row) in ly.iter().enumerate() {
        let off = row_offs(row, w, gaps[l]);
        let mid = off.last().copied().unwrap_or(0.0) * 0.5;
        row.iter().zip(&off).for_each(|(&v, &o)| cx[v] = o - mid);
        offs.push(off);
    }
    for _ in 0..10 {
        (0..nl).for_each(|i| pava_row(&ly[i], &mut cx, &offs[i], adj, 0, 1));
        (0..nl).for_each(|k| pava_row(&ly[nl - 1 - k], &mut cx, &offs[nl - 1 - k], adj, 1, 2));
        (0..nl).for_each(|i| pava_row(&ly[i], &mut cx, &offs[i], adj, 0, 3));
    }
    cx
}

/// Build placement seed with crossing-minimised row order, metrics, and initial layout.
pub(crate) fn build_seed(inp: &Input) -> Seed {
    let mut s = Seed::default();
    s.placed.px0 = PADL;
    let n = inp.bricks.len();
    if n == 0 {
        return s;
    }
    let mut used: Vec<_> = inp.bricks.iter().map(|b| b.row).collect();
    used.sort_unstable();
    used.dedup();
    let (mut lo, mut init) = (vec![0; n], vec![Vec::new(); used.len()]);
    for (i, b) in inp.bricks.iter().enumerate() {
        lo[i] = used.binary_search(&b.row).unwrap();
        init[lo[i]].push(i);
    }
    let mut map = BTreeMap::new();
    for e in &inp.edges {
        if e.a != e.b && e.a < n && e.b < n {
            let k = [(e.a, e.b), (e.b, e.a)][((lo[e.a], e.a) > (lo[e.b], e.b)) as usize];
            *map.entry(k).or_insert(0usize) += 1;
        }
    }
    let mut adj: Adj = from_fn(|_| vec![Vec::new(); n]);
    let (mut ba, mut span) = (adj.clone(), vec![false; used.len()]);
    for ((u, v), cnt) in map {
        let (ew, lu, lv, mut prev) = (cnt as f64, lo[u], lo[v], u);
        if lu == lv {
            adj[2][u].push((v, ew));
            adj[2][v].push((u, ew));
            continue;
        }
        ba[1][u].push((v, ew));
        ba[0][v].push((u, ew));
        span[(lu + 1)..=lv].fill(true);
        for l in (lu + 1)..lv {
            let d = adj[0].len();
            adj.iter_mut().for_each(|a| a.push(Vec::new()));
            init[l].push(d);
            adj[1][prev].push((d, ew));
            adj[0][d].push((prev, ew));
            prev = d;
        }
        adj[1][prev].push((v, ew));
        adj[0][v].push((prev, ew));
    }
    ba[2] = adj[2].clone();
    let fin = |x: Option<f64>| x.filter(|v| v.is_finite()).unwrap_or(0.0);
    for (i, b) in inp.bricks.iter().enumerate() {
        let wd = (b.id.chars().count() as f64 * 8.9 + 62.0).clamp(150.0, 276.0);
        let deg = |k: usize| ba[k][i].iter().map(|e| e.1 as usize).sum::<usize>();
        let p = 2.0 * PAD + PORT * deg(0).max(deg(1)).saturating_sub(1) as f64;
        s.widths.push(wd.max(p).max(fin(b.min_w)));
        s.heights.push(H.max(fin(b.min_h)));
    }
    let (mut ord, cross, cross0) = minimise(&init, &adj);
    ord.iter_mut().for_each(|r| r.retain(|&v| v < n));
    let mut y = 40.0;
    for (l, ids) in ord.iter().enumerate() {
        y += usize::from(l > 0) as f64 * [GAPY_FREE, GAPY][span[l] as usize];
        let rh = ids.iter().fold(H, |m, &i| m.max(s.heights[i]));
        s.row_y.push(y);
        s.row_h.push(rh);
        s.row_labels
            .push(inp.rows.get(used[l]).cloned().unwrap_or_default());
        y += rh;
    }
    (s.adj_bricks, s.row_order) = (ba, ord);
    (s.placed.cross, s.placed.cross0) = (cross, cross0);
    s.placed = materialize(&s, &s.row_order, None, None);
    s
}

/// Materialize non-overlapping positions for `ord`, optional `xt` left targets, and `gp` gaps.
pub(crate) fn materialize(
    s: &Seed,
    ord: &[Vec<usize>],
    xt: Option<&[f64]>,
    gp: Option<&[f64]>,
) -> Placed {
    let n = s.widths.len();
    if n == 0 {
        return s.placed.clone();
    }
    let gaps: Vec<f64> = (0..ord.len())
        .map(|l| gp.and_then(|g| g.get(l).copied()).unwrap_or(GAP).max(GAP))
        .collect();
    let cx = if let Some(xt) = xt {
        let mut x_c = vec![0.0; n];
        for (l, row) in ord.iter().enumerate() {
            let off = row_offs(row, &s.widths, gaps[l]);
            let mut t = Vec::with_capacity(row.len());
            for (k, &i) in row.iter().enumerate() {
                let x = xt.get(i).copied().filter(|v| v.is_finite());
                t.push(x.unwrap_or(s.placed.pos[i].x) + 0.5 * s.widths[i] - off[k]);
            }
            let ys = pava(&t, &vec![1.0; row.len()]);
            (0..row.len()).for_each(|k| x_c[row[k]] = ys[k] + off[k]);
        }
        x_c
    } else {
        place_x(ord, &s.widths, &s.adj_bricks, &gaps)
    };
    let min_x = (0..n).fold(f64::INFINITY, |m, i| m.min(cx[i] - 0.5 * s.widths[i]));
    let mut cross = s.placed.cross;
    if ord != s.row_order.as_slice() {
        cross = crossings(ord, &s.adj_bricks, &mut vec![(0, 0); n]);
    }
    let (mut out, mut max_x) = (Placed::default(), PADL);
    (out.px0, out.cross, out.cross0) = (PADL, cross, s.placed.cross0);
    out.pos = vec![Rect::default(); n];
    for (l, ids) in ord.iter().enumerate() {
        let (y, mut prev_r) = (s.row_y[l], f64::NEG_INFINITY);
        for &i in ids {
            let (w, h) = (s.widths[i], s.heights[i]);
            let x = (cx[i] - 0.5 * w + PADL - min_x).max(prev_r + gaps[l]);
            out.pos[i] = Rect { x, y, w, h };
            (prev_r, max_x) = (x + w, max_x.max(x + w));
        }
        let (h, label, ids) = (s.row_h[l], s.row_labels[l].clone(), ids.clone());
        out.rows.push(RowOut { y, h, label, ids });
    }
    if let Some(&y) = s.row_y.last() {
        out.h = y + s.row_h[s.row_h.len() - 1] + 50.0;
    }
    out.w = max_x + PADR;
    out
}

/// Group bricks by used rows, minimise weighted crossings, and compute non-overlapping `Rect`s.
pub(crate) fn place(inp: &Input) -> Placed {
    build_seed(inp).placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InBrick, InEdge};

    fn brick(id: &str, row: usize) -> InBrick {
        InBrick {
            id: id.into(),
            row,
            min_w: None,
            min_h: None,
        }
    }

    fn edge(a: usize, b: usize) -> InEdge {
        InEdge {
            id: format!("{a}->{b}"),
            a,
            b,
        }
    }

    fn assert_invariants(inp: &Input, p: &Placed) {
        assert_eq!(p.px0, PADL);
        assert!(p.cross <= p.cross0);
        assert_eq!(p.pos.len(), inp.bricks.len());
        if inp.bricks.is_empty() {
            return;
        }
        let min_x = p.pos.iter().fold(f64::INFINITY, |m, r| m.min(r.x));
        let max_x = p
            .pos
            .iter()
            .fold(f64::NEG_INFINITY, |m, r| m.max(r.x + r.w));
        assert!((min_x - PADL).abs() < 1e-9);
        assert!((p.w - (max_x + PADR)).abs() < 1e-9);
        let mut seen = vec![0usize; inp.bricks.len()];
        for (l, row) in p.rows.iter().enumerate() {
            if l > 0 {
                assert!(p.rows[l - 1].y + p.rows[l - 1].h < row.y);
            }
            for (k, &i) in row.ids.iter().enumerate() {
                seen[i] += 1;
                assert!((p.pos[i].y - row.y).abs() < 1e-9);
                assert!(p.pos[i].h <= row.h + 1e-9);
                if k > 0 {
                    let prev = p.pos[row.ids[k - 1]];
                    assert!(prev.x + prev.w + GAP <= p.pos[i].x + 1e-9);
                }
            }
        }
        assert!(seen.iter().all(|&c| c == 1));
        let last = p.rows.last().unwrap();
        assert!((p.h - (last.y + last.h + 50.0)).abs() < 1e-9);
    }

    #[test]
    fn pava_weights_and_ties() {
        assert!(pava(&[], &[]).is_empty());
        assert_eq!(
            pava(&[1.0, 3.0, 7.0], &[1.0, 1.0, 1.0]),
            vec![1.0, 3.0, 7.0]
        );
        assert_eq!(pava(&[10.0, 2.0], &[1.0, 3.0]), vec![4.0, 4.0]);
        assert_eq!(
            pava(&[50.0, 0.0, 60.0], &[1.0, 0.0, 1.0]),
            vec![50.0, 50.0, 60.0]
        );
        assert_eq!(pava(&[10.0, 2.0], &[0.0, 0.0]), vec![6.0, 6.0]);
    }

    #[test]
    fn empty_single_and_min_bounds() {
        let emp = place(&Input::default());
        assert_invariants(&Input::default(), &emp);
        assert_eq!((emp.w, emp.h, emp.rows.len()), (0.0, 0.0, 0));

        let mut bricks = vec![brick("a", 0), brick(&"x".repeat(40), 0), brick("grp", 1)];
        bricks[2].min_w = Some(360.0);
        bricks[2].min_h = Some(100.0);
        bricks[0].min_w = Some(f64::NAN);
        bricks[0].min_h = Some(10.0);
        let mut edges = Vec::new();
        for k in 0..25 {
            bricks.push(brick(&format!("c{k}"), 2));
            edges.push(edge(2, 3 + k));
        }
        let inp = Input {
            bricks,
            rows: vec!["r0".into(), "r1".into(), "r2".into()],
            edges,
        };
        let p = place(&inp);
        assert_invariants(&inp, &p);
        assert!((p.pos[0].w - 150.0).abs() < 1e-9);
        assert!((p.pos[0].h - H).abs() < 1e-9);
        assert!((p.pos[1].w - 276.0).abs() < 1e-9);
        let port_w = 2.0 * PAD + PORT * 24.0;
        assert!((p.pos[2].w - 360.0_f64.max(port_w)).abs() < 1e-9);
        assert!((p.pos[2].h - 100.0).abs() < 1e-9);
        assert!((p.rows[1].h - 100.0).abs() < 1e-9);
    }

    #[test]
    fn row_grouping_and_free_vs_spanned_gap() {
        let inp = Input {
            rows: vec![
                "r0".into(),
                "unused1".into(),
                "r2".into(),
                "unused3".into(),
                "r4".into(),
            ],
            bricks: vec![brick("a", 0), brick("b", 2), brick("c", 4)],
            edges: vec![edge(0, 1)],
        };
        let p = place(&inp);
        assert_invariants(&inp, &p);
        assert_eq!(p.rows.len(), 3);
        assert_eq!(
            p.rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            vec!["r0", "r2", "r4"]
        );
        assert!((p.rows[1].y - (p.rows[0].y + p.rows[0].h + GAPY)).abs() < 1e-9);
        assert!((p.rows[2].y - (p.rows[1].y + p.rows[1].h + GAPY_FREE)).abs() < 1e-9);

        let skip = Input {
            edges: vec![edge(0, 2)],
            ..inp
        };
        let ps = place(&skip);
        assert!((ps.rows[1].y - (ps.rows[0].y + ps.rows[0].h + GAPY)).abs() < 1e-9);
        assert!((ps.rows[2].y - (ps.rows[1].y + ps.rows[1].h + GAPY)).abs() < 1e-9);
    }

    #[test]
    fn crossing_minimisation_and_weighted_bundles() {
        let tangle = Input {
            rows: vec!["top".into(), "mid".into(), "bot".into()],
            bricks: vec![
                brick("t0", 0),
                brick("t1", 0),
                brick("m0", 1),
                brick("b0", 2),
                brick("b1", 2),
            ],
            edges: vec![edge(0, 2), edge(2, 4), edge(1, 3)],
        };
        let pt = place(&tangle);
        assert_invariants(&tangle, &pt);
        assert!(pt.cross0 > 0.0);
        assert_eq!(pt.cross, 0.0);
        assert_eq!(place(&tangle).pos, pt.pos);

        let weighted = Input {
            rows: vec!["top".into(), "bot".into()],
            bricks: vec![
                brick("u0", 0),
                brick("u1", 0),
                brick("v0", 1),
                brick("v1", 1),
            ],
            edges: vec![edge(0, 2), edge(1, 3), edge(0, 3), edge(0, 3), edge(0, 3)],
        };
        let pw = place(&weighted);
        assert_invariants(&weighted, &pw);
        assert!(pw.cross <= pw.cross0);

        let ordered = Input {
            rows: vec!["top".into(), "bot".into()],
            bricks: vec![
                brick("u0", 0),
                brick("u1", 0),
                brick("v0", 1),
                brick("v1", 1),
            ],
            edges: vec![edge(0, 2), edge(1, 3)],
        };
        let po = place(&ordered);
        assert_eq!((po.cross0, po.cross), (0.0, 0.0));
        assert_eq!(po.rows[0].ids, vec![0, 1]);
        assert_eq!(po.rows[1].ids, vec![2, 3]);
    }

    #[test]
    fn vertical_alignment_and_fixture_invariants() {
        let chain = Input {
            rows: vec!["a".into(), "b".into(), "c".into()],
            bricks: vec![
                brick("left_wide_name_here", 0),
                brick("right", 0),
                brick("m", 1),
                brick("x", 2),
            ],
            edges: vec![edge(1, 2), edge(2, 3)],
        };
        let pc = place(&chain);
        assert_invariants(&chain, &pc);
        let cx = |i: usize| pc.pos[i].x + pc.pos[i].w * 0.5;
        assert!((cx(1) - cx(2)).abs() < 1e-6);
        assert!((cx(2) - cx(3)).abs() < 1e-6);

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
        let fix = Input {
            rows: vec![
                "port".into(),
                "core".into(),
                "adapter".into(),
                "assembly".into(),
            ],
            bricks: names.iter().map(|&(id, r)| brick(id, r)).collect(),
            edges: pairs
                .iter()
                .enumerate()
                .map(|(k, &(a, b))| InEdge {
                    id: format!("e{k}"),
                    a,
                    b,
                })
                .collect(),
        };
        let pf = place(&fix);
        assert_invariants(&fix, &pf);
        assert!(pf.cross <= pf.cross0);
    }

    #[test]
    fn same_row_attraction_places_connected_neighbors_adjacent() {
        let one_row = Input {
            rows: vec!["r0".into()],
            bricks: vec![brick("a", 0), brick("b", 0), brick("c", 0)],
            edges: vec![edge(0, 2)],
        };
        let p1 = place(&one_row);
        assert_invariants(&one_row, &p1);
        let pos0 = p1.rows[0].ids.iter().position(|&x| x == 0).unwrap();
        let pos2 = p1.rows[0].ids.iter().position(|&x| x == 2).unwrap();
        assert_eq!(pos0.abs_diff(pos2), 1, "ids={:?}", p1.rows[0].ids);
        assert_eq!(p1.cross, 0.0);

        let two_rows = Input {
            rows: vec!["r0".into(), "r1".into()],
            bricks: vec![
                brick("a", 0),
                brick("b", 0),
                brick("c", 0),
                brick("d0", 1),
                brick("d1", 1),
            ],
            edges: vec![edge(0, 3), edge(2, 4), edge(0, 2)],
        };
        let p2 = place(&two_rows);
        assert_invariants(&two_rows, &p2);
        let q0 = p2.rows[0].ids.iter().position(|&x| x == 0).unwrap();
        let q2 = p2.rows[0].ids.iter().position(|&x| x == 2).unwrap();
        assert_eq!(q0.abs_diff(q2), 1, "ids={:?}", p2.rows[0].ids);
    }

    #[test]
    fn seed_and_materialize_enforce_pava_gaps_and_min_x() {
        let inp = Input {
            rows: vec!["top".into(), "bot".into()],
            bricks: vec![brick("a", 0), brick("b", 0), brick("c", 1), brick("d", 1)],
            edges: vec![edge(0, 2), edge(1, 3)],
        };
        let seed = build_seed(&inp);
        assert_eq!(seed.row_order, vec![vec![0, 1], vec![2, 3]]);
        assert_eq!(seed.placed.pos, place(&inp).pos);

        let swapped = vec![vec![1, 0], vec![2, 3]];
        let ps = materialize(&seed, &swapped, None, None);
        assert_eq!(ps.rows[0].ids, vec![1, 0]);
        assert!(ps.cross > seed.placed.cross);
        assert!((ps.pos.iter().fold(f64::INFINITY, |m, r| m.min(r.x)) - PADL).abs() < 1e-9);

        let targets = vec![200.0, 210.0, 220.0, 225.0];
        let pt = materialize(&seed, &seed.row_order, Some(&targets), Some(&[36.0, 24.0]));
        assert!((pt.pos[0].x + pt.pos[0].w + 36.0 - pt.pos[1].x).abs() < 1e-6);
        assert!((pt.pos[2].x + pt.pos[2].w + 24.0 - pt.pos[3].x).abs() < 1e-6);
        let min_x = pt.pos.iter().fold(f64::INFINITY, |m, r| m.min(r.x));
        assert!((min_x - PADL).abs() < 1e-9);
    }
}
