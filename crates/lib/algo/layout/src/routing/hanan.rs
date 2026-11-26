//! Unified Hanan Strip Graph H(X) with O(1) bitmask clearance queries.

use super::{BOT_CLEAR, CLEAR, inset, side_span, wall_coord};
use crate::{Rect, Side};

const TIGHT: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClearMode {
    Normal = 0,
    Tight = 1,
    Overlap = 2,
}

pub(super) type Ctx = (usize, usize, ClearMode);

pub(super) struct HananGrid {
    pub(super) lines: [Vec<f64>; 2],
    pub(super) mids: [Vec<f64>; 2],
    pub(super) spans: [Vec<f64>; 2],
    pub(super) dims: [usize; 2],
    words: usize,
    obs: [Vec<u64>; 3],
    pos: Vec<Rect>,
}

impl HananGrid {
    pub(super) fn build(pos: &[Rect], active: &[bool]) -> Self {
        let f = active_frame(pos, active);
        let (mut rx, mut ry) = (vec![f[0], f[1]], vec![f[2], f[3]]);
        for r in pos {
            let [x0, x1, y0, y1] = r.bb();
            if !(r.x + r.y).is_finite() || x1 < f[0] || x0 > f[1] || y1 < f[2] || y0 > f[3] {
                continue;
            }
            rx.extend([x0 - CLEAR, x0, x1, x1 + CLEAR].map(|v| v.clamp(f[0], f[1])));
            ry.extend([y0 - CLEAR, y0, y1, y1 + BOT_CLEAR].map(|v| v.clamp(f[2], f[3])));
        }
        let lines = [dedup_coords(rx, 260), dedup_coords(ry, 140)];
        let dims = [lines[0].len() - 1, lines[1].len() - 1];
        let (nc, words) = (dims[0] * dims[1], pos.len().div_ceil(64).max(1));
        let mid = |k: usize| lines[k].windows(2).map(|w| (w[0] + w[1]) * 0.5).collect();
        let (zf, zu) = (vec![0.0; nc], vec![0u64; nc * words]);
        let mut g = Self {
            mids: [mid(0), mid(1)],
            spans: [zf.clone(), zf],
            obs: [zu.clone(), zu.clone(), zu],
            pos: pos.to_vec(),
            lines,
            dims,
            words,
        };
        for (bi, r) in pos.iter().enumerate() {
            let (w, bit) = (bi / 64, 1u64 << (bi % 64));
            for (kind, p, bot) in [(0, CLEAR, BOT_CLEAR), (1, TIGHT, TIGHT), (2, 0.0, 0.0)] {
                let xr = mid_range(&g.mids[0], r.x - p, r.x + r.w + p);
                for cy in mid_range(&g.mids[1], r.y - p, r.y + r.h + bot) {
                    for cx in xr.clone() {
                        g.obs[kind][(cy * dims[0] + cx) * words + w] |= bit;
                    }
                }
            }
        }
        g.fill_spans();
        g
    }

    fn fill_spans(&mut self) {
        let (w, o) = (self.words, &self.obs);
        for ax in 0..2 {
            for j in 0..self.dims[ax ^ 1] {
                let mut i = 0;
                while i < self.dims[ax] {
                    let (c0, mut end) = (self.cell_ax(ax, i, j) * w, i + 1);
                    while end < self.dims[ax] {
                        let ce = self.cell_ax(ax, end, j) * w;
                        if o[0][c0..c0 + w] != o[0][ce..ce + w]
                            || o[2][c0..c0 + w] != o[2][ce..ce + w]
                        {
                            break;
                        }
                        end += 1;
                    }
                    let span = self.lines[ax][end] - self.lines[ax][i];
                    for k in i..end {
                        let c = self.cell_ax(ax, k, j);
                        self.spans[ax][c] = span;
                    }
                    i = end;
                }
            }
        }
    }

    #[inline]
    pub(super) fn cell(&self, cx: usize, cy: usize) -> usize {
        cy * self.dims[0] + cx
    }

    #[inline]
    pub(super) fn cell_ax(&self, ax: usize, p: usize, q: usize) -> usize {
        let cw = self.dims[0];
        [q * cw + p, p * cw + q][ax]
    }

    #[inline]
    pub(super) fn cap(&self, ax: usize, c: usize, p: usize, hs: f64) -> f64 {
        (self.lines[ax][p + 1] - self.lines[ax][p]).max(self.spans[ax][c].min(2.0 * hs))
    }

    pub(super) fn cell_free(&self, c: usize, (a, b, m): Ctx) -> bool {
        if m == ClearMode::Overlap {
            return true;
        }
        let (base, w) = (c * self.words, self.words);
        !has_mask(&self.obs[m as usize][base..base + w], a, b, true)
            && !has_mask(&self.obs[2][base..base + w], a, b, false)
    }

    pub(super) fn ov_pen(&self, c: usize, (a, b, m): Ctx) -> i64 {
        if m != ClearMode::Overlap {
            return 0;
        }
        let (base, w) = (c * self.words, self.words);
        if self.obs[2][base..base + w].iter().any(|&v| v != 0) {
            return 100_000;
        }
        20_000 * i64::from(has_mask(&self.obs[1][base..base + w], a, b, true))
    }

    pub(super) fn wall_cell(&self, bi: usize, sd: Side) -> Option<usize> {
        let fax = usize::from(matches!(sd, Side::T | Side::B));
        let bef = matches!(sd, Side::T | Side::L);
        let wall = wall_coord(&self.pos[bi], sd);
        let p = self.mids[fax].partition_point(|&m| if bef { m < wall } else { m <= wall });
        ((!bef || p > 0) && (bef || p < self.dims[fax])).then(|| p - usize::from(bef))
    }

    pub(super) fn side_cells(&self, bx: (usize, Side, f64), ctx: Ctx) -> Vec<usize> {
        let (bi, sd, hs) = bx;
        let Some(fixed) = self.wall_cell(bi, sd) else {
            return Vec::new();
        };
        let sax = usize::from(!matches!(sd, Side::T | Side::B));
        let (slo, shi) = side_span(&self.pos[bi], sd, hs);
        let mut out = Vec::new();
        for v in 0..self.dims[sax] {
            let c = self.cell_ax(sax, v, fixed);
            if overlaps_span(self.lines[sax][v], self.lines[sax][v + 1], slo, shi)
                && self.cell_free(c, ctx)
            {
                out.push(c);
            }
        }
        out
    }

    pub(super) fn expand_slab(
        &self,
        ax: usize,
        ps: (usize, usize),
        al: (usize, usize),
        ctx: Ctx,
        end: Option<(usize, Side, f64)>,
    ) -> (usize, usize) {
        let (mut p0, mut p1) = ps;
        let sp = end.map(|(bx, sd, hs)| side_span(&self.pos[bx], sd, hs));
        let ok = |p: usize| {
            let in_s = sp.is_none_or(|(s0, s1)| {
                overlaps_span(self.lines[ax][p], self.lines[ax][p + 1], s0, s1)
            });
            in_s && (al.0..=al.1).all(|q| self.cell_free(self.cell_ax(ax, p, q), ctx))
        };
        while p0 > 0 && ok(p0 - 1) {
            p0 -= 1;
        }
        while p1 + 1 < self.dims[ax] && ok(p1 + 1) {
            p1 += 1;
        }
        (p0, p1)
    }

    pub(super) fn exact_bounds(
        &self,
        ax: usize,
        ps: (usize, usize),
        al: (usize, usize),
        (a, b, m): Ctx,
        hs: f64,
    ) -> (f64, f64) {
        let (ax_o, mut lo, mut hi) = (ax ^ 1, self.lines[ax][ps.0], self.lines[ax][ps.1 + 1]);
        if m != ClearMode::Overlap {
            let (sp0, sp1) = (self.lines[ax_o][al.0], self.lines[ax_o][al.1 + 1]);
            let (mid_a, d) = ((sp0 + sp1) * 0.5, ((sp1 - sp0) * 0.5 - 0.5).max(0.1));
            let (ax0, ax1, mid) = (mid_a - d, mid_a + d, (lo + hi) * 0.5);
            for (j, r) in self.pos.iter().enumerate() {
                let (cp, cb) = match (j == a || j == b, m) {
                    (true, _) => (0.0, 0.0),
                    (false, ClearMode::Tight) => (TIGHT, TIGHT),
                    (false, _) => (CLEAR, BOT_CLEAR),
                };
                let bx = [[r.x - cp, r.x + r.w + cp], [r.y - cp, r.y + r.h + cb]];
                if bx[ax_o][1] > ax0 && bx[ax_o][0] < ax1 {
                    if bx[ax][1] <= mid {
                        lo = lo.max(bx[ax][1]);
                    }
                    if bx[ax][0] >= mid {
                        hi = hi.min(bx[ax][0]);
                    }
                }
            }
        }
        inset(lo, hi, hs)
    }
}

#[inline]
pub(super) fn qpx(v: f64) -> i64 {
    (v * 10.0).round() as i64
}

pub(super) fn overlaps_span(c0: f64, c1: f64, slo: f64, shi: f64) -> bool {
    if shi - slo <= 1.0 {
        c0 <= shi + 0.5 && c1 >= slo - 0.5
    } else {
        c1 > slo + 0.5 && c0 < shi - 0.5
    }
}

fn active_frame(pos: &[Rect], active: &[bool]) -> [f64; 4] {
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (x0, x1);
    for (r, &act) in pos.iter().zip(active) {
        if act && (r.x + r.y).is_finite() {
            let [a0, a1, b0, b1] = r.bb();
            (x0, x1, y0, y1) = (x0.min(a0), x1.max(a1), y0.min(b0), y1.max(b1));
        }
    }
    if !x0.is_finite() {
        return [0.0, 100.0, 0.0, 100.0];
    }
    let m = CLEAR + 24.0;
    [x0 - m, x1 + m, y0 - m, y1 + BOT_CLEAR + 24.0]
}

fn dedup_coords(mut raw: Vec<f64>, max_len: usize) -> Vec<f64> {
    raw.sort_by(f64::total_cmp);
    let (last, mut tol) = (*raw.last().unwrap_or(&0.0), 1.0);
    loop {
        let mut out = vec![raw[0]];
        for &v in &raw[1..raw.len().saturating_sub(1)] {
            if v - *out.last().unwrap() > tol {
                out.push(v);
            }
        }
        if last - *out.last().unwrap() > tol {
            out.push(last);
        } else if out.len() > 1 {
            *out.last_mut().unwrap() = last;
        }
        if out.len() <= max_len || tol >= 64.0 {
            return out;
        }
        tol *= 1.5;
    }
}

fn mid_range(mids: &[f64], lo: f64, hi: f64) -> std::ops::Range<usize> {
    let i0 = mids.partition_point(|&m| m <= lo);
    i0..mids.partition_point(|&m| m < hi).max(i0)
}

fn has_mask(mask: &[u64], a: usize, b: usize, excl: bool) -> bool {
    if !excl {
        return (mask[a / 64] & (1 << (a % 64))) | (mask[b / 64] & (1 << (b % 64))) != 0;
    }
    mask.iter().enumerate().any(|(w, &bits)| {
        let clr = |k: usize| if w == k / 64 { 1u64 << (k % 64) } else { 0 };
        (bits & !(clr(a) | clr(b))) != 0
    })
}
