//! Negotiated Congestion A* (PathFinder) on H(X) with 0-bend fast certificate.

use super::hanan::{ClearMode, HananGrid, overlaps_span, qpx};
use super::{Bundle, SegRoute, TRACK_GAP, side_center, side_span, wall_coord};
use crate::{Rect, Side};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const W_BEND: i64 = 3200;
const W_CROSS: i64 = 4500;
const W_POL: i64 = 4500;
const EXIT_S: [Side; 4] = [Side::T, Side::R, Side::B, Side::L];
const ENTER_S: [Side; 4] = [Side::B, Side::L, Side::T, Side::R];
const STEPS: [(isize, isize); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

type BestKey = (usize, i64, usize, usize, i64);
type Heap = BinaryHeap<Reverse<(i64, u32)>>;
type Pack<'a> = (&'a [Rect], &'a Bundle, &'a Occ);
struct Occ([Vec<f64>; 2], [Vec<u16>; 2], [Vec<i64>; 2]);
struct Sc(Vec<(i64, u32, u32)>, Vec<(u32, u8)>, u32, Heap);

#[derive(Clone)]
struct Routed(SegRoute, Vec<(usize, usize)>, (usize, usize, i64));

impl Sc {
    fn relax(&mut self, st: usize, g: i64, par: u32, h: i64) {
        if self.0[st].1 <= self.2 && (self.0[st].1 < self.2 || g < self.0[st].0) {
            self.0[st] = (g, self.2, par);
            self.3.push(Reverse((g + h, st as u32)));
        }
    }
}

impl Occ {
    fn apply(&mut self, cs: &[(usize, usize)], hs: f64, add: bool) {
        let d = (2.0 * hs + TRACK_GAP) * if add { 1.0 } else { -1.0 };
        let s = if add { 1 } else { -1 };
        for &(c, ax) in cs {
            self.0[ax][c] = (self.0[ax][c] + d).max(0.0);
            self.1[ax][c] = (self.1[ax][c] as i32 + s).max(0) as u16;
        }
    }

    fn bump(&mut self, g: &HananGrid) -> (usize, i64) {
        let (mut cells, mut q, cw) = (0, 0, g.dims[0]);
        for c in 0..(cw * g.dims[1]) {
            for (ax, p) in [(0, c % cw), (1, c / cw)] {
                let cap = g.lines[ax][p + 1] - g.lines[ax][p];
                if self.1[ax][c] > 1 && self.0[ax][c] - TRACK_GAP > cap + 0.5 {
                    let ov = qpx(self.0[ax][c] - TRACK_GAP - cap);
                    self.2[ax][c] = (self.2[ax][c] + ov).min(1_000_000);
                    (cells, q) = (cells + 1, q + ov);
                }
            }
        }
        (cells, q)
    }

    fn cost(&self, g: &HananGrid, c: usize, ax: usize, hs: f64, p_it: i64) -> i64 {
        let p = [c % g.dims[0], c / g.dims[0]][ax];
        let (occ, cap) = (self.0[ax][c], g.cap(ax, c, p, hs));
        let ov = (occ + 2.0 * hs - cap).max(0.0);
        let cong = i64::from(ov > 0.0 || occ > 0.0) * ((ov * p_it as f64) as i64 + self.2[ax][c]);
        self.1[ax ^ 1][c] as i64 * W_CROSS + cong
    }
}

pub(super) fn route(g: &HananGrid, pos: &[Rect], bs: &[Bundle], fast: bool) -> Vec<SegRoute> {
    let nc = g.dims[0] * g.dims[1];
    let (v, z, h) = (vec![0.0; nc], vec![0u16; nc], vec![0i64; nc]);
    let mut occ = Occ([v.clone(), v], [z.clone(), z], [h.clone(), h]);
    let (d, g0) = (vec![(0, 0, u32::MAX); nc * 4], vec![(0, 0); nc]);
    let mut sc = Sc(d, g0, 0, BinaryHeap::new());
    let mut cur: Vec<Option<Routed>> = vec![None; bs.len()];
    let mut best: Option<(BestKey, Vec<SegRoute>)> = None;
    for pass in 0..if fast { 2 } else { 5 } {
        let (p_it, mut chg) = (20i64 << pass.min(4), false);
        for (bi, b) in bs.iter().enumerate() {
            if let Some(p) = &cur[bi] {
                occ.apply(&p.1, b.hs, false);
            }
            let mut rb = try_zero(g, pos, b, &occ);
            for m in [ClearMode::Normal, ClearMode::Tight, ClearMode::Overlap] {
                rb = rb.or_else(|| astar(g, (pos, b, &occ), &mut sc, m, p_it));
            }
            let rb =
                rb.unwrap_or_else(|| Routed(SegRoute::fallback(pos, b), Vec::new(), (10, 2, 1000)));
            occ.apply(&rb.1, b.hs, true);
            chg |= cur[bi].as_ref().is_none_or(|p| p.1 != rb.1);
            cur[bi] = Some(rb);
        }
        let (oc, oq) = occ.bump(g);
        let key = cur.iter().flatten().fold((oc, oq, 0, 0, 0i64), |a, r| {
            (a.0, a.1, a.2 + r.2.0, a.3 + r.2.1, a.4 + r.2.2)
        });
        if best.as_ref().is_none_or(|(bk, _)| key < *bk) {
            best = Some((key, cur.iter().flatten().map(|r| r.0.clone()).collect()));
        }
        if oc == 0 && (pass == 0 || !chg) {
            break;
        }
    }
    best.map(|(_, r)| r).unwrap_or_default()
}

fn try_zero(g: &HananGrid, pos: &[Rect], b: &Bundle, occ: &Occ) -> Option<Routed> {
    let (ra, rb, ctx) = (&pos[b.a], &pos[b.b], (b.a, b.b, ClearMode::Normal));
    let sides = [
        (Side::B, Side::T, 0, rb.y >= ra.y + ra.h),
        (Side::T, Side::B, 0, ra.y >= rb.y + rb.h),
        (Side::R, Side::L, 1, rb.x >= ra.x + ra.w),
        (Side::L, Side::R, 1, ra.x >= rb.x + rb.w),
    ];
    for (sa, sb, ax, ok) in sides {
        if !ok {
            continue;
        }
        let ((sa0, sa1), (sb0, sb1)) = (side_span(ra, sa, b.hs), side_span(rb, sb, b.hs));
        let (olo, ohi) = (sa0.max(sb0), sa1.min(sb1));
        if olo > ohi + 1e-6 {
            continue;
        }
        let (Some(va), Some(vb)) = (g.wall_cell(b.a, sa), g.wall_cell(b.b, sb)) else {
            continue;
        };
        let (a0, a1) = (va.min(vb), va.max(vb));
        let tgt = ((side_center(ra, sa) + side_center(rb, sb)) * 0.5).clamp(olo, ohi);
        let dist = |p: usize| (g.mids[ax][p] - tgt).abs();
        let p = (0..g.dims[ax])
            .filter(|&p| {
                overlaps_span(g.lines[ax][p], g.lines[ax][p + 1], olo, ohi)
                    && (a0..=a1).all(|q| {
                        let c = g.cell_ax(ax, p, q);
                        g.cell_free(c, ctx)
                            && occ.1[ax ^ 1][c] == 0
                            && occ.0[ax][c] + 2.0 * b.hs <= g.cap(ax, c, p, b.hs) + 0.5
                    })
            })
            .min_by(|&p0, &p1| dist(p0).total_cmp(&dist(p1)));
        let Some(p) = p else { continue };
        let ep = g.expand_slab(ax, (p, p), (a0, a1), ctx, None);
        let (lo, hi) = g.exact_bounds(ax, ep, (a0, a1), ctx, b.hs);
        let cs = (a0..=a1).map(|q| (g.cell_ax(ax, p, q), ax)).collect();
        let sr = SegRoute::from_slabs(pos, b, (sa, sb), &[(ax == 1, lo, hi, tgt)]);
        let len = qpx((wall_coord(rb, sb) - wall_coord(ra, sa)).abs());
        return Some(Routed(sr, cs, (0, 0, len)));
    }
    None
}

fn astar(g: &HananGrid, cb: Pack<'_>, sc: &mut Sc, m: ClearMode, p_it: i64) -> Option<Routed> {
    let ((pos, b, occ), cw, ch) = (cb, g.dims[0], g.dims[1]);
    sc.2 += 2;
    sc.3.clear();
    let (cur, ra, rb, ctx) = (sc.2, &pos[b.a], &pos[b.b], (b.a, b.b, m));
    let (mx, my) = (&g.mids[0], &g.mids[1]);
    let mut gb = [i64::MAX, i64::MIN, i64::MAX, i64::MIN];
    for (d, sb) in ENTER_S.into_iter().enumerate() {
        for c in g.side_cells((b.b, sb, b.hs), ctx) {
            let e = &mut sc.1[c];
            *e = (cur, if e.0 == cur { e.1 } else { 0 } | (1 << d));
            let (qx, qy) = (qpx(mx[c % cw]), qpx(my[c / cw]));
            gb = [gb[0].min(qx), gb[1].max(qx), gb[2].min(qy), gb[3].max(qy)];
        }
    }
    if gb[0] == i64::MAX {
        return None;
    }
    let (b_dn, b_up) = (rb.y >= ra.y + ra.h - 0.5, ra.y >= rb.y + rb.h - 0.5);
    let heur = |cx: usize, cy: usize| {
        let (qx, qy) = (qpx(mx[cx]), qpx(my[cy]));
        (gb[0] - qx).max(0).max(qx - gb[1]) + (gb[2] - qy).max(0).max(qy - gb[3])
    };
    let pol = |sd: Side, bad: Side| match (b_dn || b_up, sd) {
        (true, s) if s == bad => W_POL,
        (true, Side::L | Side::R) => 50,
        _ => 0,
    };
    for (d, sa) in EXIT_S.into_iter().enumerate() {
        let ep = pol(sa, if b_dn { Side::T } else { Side::B });
        for c in g.side_cells((b.a, sa, b.hs), ctx) {
            let cost = ep + occ.cost(g, c, d & 1, b.hs, p_it) + g.ov_pen(c, ctx);
            sc.relax(c * 4 + d, cost, u32::MAX, heur(c % cw, c / cw));
        }
    }
    let mut best_g: Option<(i64, usize)> = None;
    while let Some(Reverse((f, st_u))) = sc.3.pop() {
        if best_g.is_some_and(|(bg, _)| f >= bg) {
            break;
        }
        let st = st_u as usize;
        if sc.0[st].1 == cur + 1 {
            continue;
        }
        sc.0[st].1 = cur + 1;
        let (g0, c, d, cx, cy) = (sc.0[st].0, st / 4, st % 4, (st / 4) % cw, (st / 4) / cw);
        if sc.1[c].0 == cur && (sc.1[c].1 & (1 << d)) != 0 {
            let tot = g0 + pol(ENTER_S[d], if b_dn { Side::B } else { Side::T });
            if best_g.is_none_or(|(bg, _)| tot < bg) {
                best_g = Some((tot, st));
            }
        }
        for (nd, &(dx, dy)) in STEPS.iter().enumerate() {
            let (nx, ny) = (cx.wrapping_add_signed(dx), cy.wrapping_add_signed(dy));
            if (nd ^ 2) == d || nx >= cw || ny >= ch {
                continue;
            }
            let nc = g.cell(nx, ny);
            if !g.cell_free(nc, ctx) {
                continue;
            }
            let step = (qpx(mx[nx]) - qpx(mx[cx])).abs() + (qpx(my[ny]) - qpx(my[cy])).abs();
            let mul = 1 + 2 * i64::from((b_dn && nd == 0) || (b_up && nd == 2));
            let oc = occ.cost(g, nc, nd & 1, b.hs, p_it) + g.ov_pen(nc, ctx);
            let ng = g0 + step * mul + W_BEND * i64::from(nd != d) + oc;
            sc.relax(nc * 4 + nd, ng, st_u, heur(nx, ny));
        }
    }
    let (_, mut st) = best_g?;
    let (sb, mut chain) = (ENTER_S[st % 4], Vec::new());
    loop {
        chain.push(((st / 4) % cw, (st / 4) / cw, st % 4));
        if sc.0[st].2 == u32::MAX {
            break;
        }
        st = sc.0[st].2 as usize;
    }
    chain.reverse();
    Some(collapse_path(g, cb, (EXIT_S[st % 4], sb, m), &chain))
}

fn collapse_path(
    g: &HananGrid,
    (pos, b, occ): Pack<'_>,
    (sa, sb, m): (Side, Side, ClearMode),
    chain: &[(usize, usize, usize)],
) -> Routed {
    let mut grps: Vec<[usize; 6]> = Vec::new();
    for &(cx, cy, dir) in chain {
        let (ax, (p, a)) = (dir & 1, if dir & 1 == 1 { (cy, cx) } else { (cx, cy) });
        if let Some(l) = grps.last_mut()
            && l[0] == ax
        {
            (l[1], l[2], l[3], l[4]) = (l[1].min(p), l[2].max(p), l[3].min(a), l[4].max(a));
        } else {
            grps.push([ax, p, p, a, a, p]);
        }
    }
    let along = |gs: &[[usize; 6]], i: usize| {
        let p = i.checked_sub(1).map(|k| gs[k]);
        let (l0, h0) = p.map_or((usize::MAX, 0), |s| (s[1], s[2]));
        let (l1, h1) = gs.get(i + 1).map_or((usize::MAX, 0), |s| (s[1], s[2]));
        (gs[i][3].min(l0).min(l1), gs[i][4].max(h0).max(h1))
    };
    let wid = |i: usize| g.lines[grps[i][0]][grps[i][2] + 1] - g.lines[grps[i][0]][grps[i][1]];
    let mut order: Vec<usize> = (0..grps.len()).collect();
    order.sort_by(|&i, &j| wid(i).total_cmp(&wid(j)).then(i.cmp(&j)));
    for i in order {
        let [ax, p0, p1, ..] = grps[i];
        let end = match i {
            0 => Some((b.a, sa, b.hs)),
            _ if i + 1 == grps.len() => Some((b.b, sb, b.hs)),
            _ => None,
        };
        let (c0, c1) = g.expand_slab(ax, (p0, p1), along(&grps, i), (b.a, b.b, m), end);
        (grps[i][1], grps[i][2]) = (c0, c1);
    }
    let slabs: Vec<_> = (0..grps.len())
        .map(|i| {
            let [ax, p0, p1, _, _, init_p] = grps[i];
            let (lo, hi) = g.exact_bounds(ax, (p0, p1), along(&grps, i), (b.a, b.b, m), b.hs);
            (ax == 1, lo, hi, g.mids[ax][init_p])
        })
        .collect();
    let (mut cs, mut cross_e, mut len_f) = (Vec::with_capacity(chain.len()), 0, 0.0);
    let ((mx, my), mut prev) = ((&g.mids[0], &g.mids[1]), chain[0]);
    for &(cx, cy, d) in chain {
        let c = g.cell(cx, cy);
        cs.push((c, d & 1));
        cross_e += occ.1[(d & 1) ^ 1][c] as usize;
        len_f += (mx[cx] - mx[prev.0]).abs() + (my[cy] - my[prev.1]).abs();
        prev = (cx, cy, d);
    }
    let sr = SegRoute::from_slabs(pos, b, (sa, sb), &slabs);
    Routed(sr, cs, (cross_e, grps.len() - 1, qpx(len_f)))
}
