//! Unified Hanan-Strip PathFinder + Segments-as-Variables edge router.

mod hanan;
mod pathfinder;
mod solve;

use crate::{End, Port, Pt, Rect, Route, Side};
use std::collections::{BTreeMap, HashMap};

pub(super) const PAD: f64 = 10.0;
pub(super) const CLEAR: f64 = 8.0;
pub(super) const BOT_CLEAR: f64 = 16.0;
pub(super) const TRACK_GAP: f64 = 10.0;
pub(super) const BUNDLE_PITCH: f64 = 8.0;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Energy {
    pub(crate) hard: f64,
    #[allow(dead_code)]
    pub(crate) crossings: usize,
    #[allow(dead_code)]
    pub(crate) bends: usize,
    #[allow(dead_code)]
    pub(crate) length: f64,
    pub(crate) total: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BundleStat {
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) weight: usize,
    pub(crate) bends: usize,
    pub(crate) crossings: usize,
    /// Port X coords `(port_a_x, port_b_x)` when both ends attach on `Side::T/B`.
    pub(crate) v_align: Option<(f64, f64)>,
}

#[derive(Default)]
pub(crate) struct RouteOut {
    pub(crate) routes: Vec<Option<Route>>,
    pub(crate) po: HashMap<(usize, End), Port>,
    pub(crate) energy: Energy,
    pub(crate) stats: Vec<BundleStat>,
}

#[derive(Clone, Debug)]
pub(super) struct Bundle {
    pub(super) a: usize,
    pub(super) b: usize,
    pub(super) edges: Vec<usize>,
    pub(super) hs: f64,
}

#[derive(Clone, Debug)]
pub(super) struct SegVar {
    pub(super) is_horiz: bool,
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) span: [f64; 2],
    pub(super) u: f64,
    pub(super) target_u: f64,
    pub(super) attach: [Option<(usize, Side)>; 2],
}

#[derive(Clone, Debug)]
pub(super) struct SegRoute {
    pub(super) a: usize,
    pub(super) b: usize,
    pub(super) weight: usize,
    pub(super) hs: f64,
    pub(super) sa: Side,
    pub(super) sb: Side,
    pub(super) segs: Vec<SegVar>,
}

impl SegRoute {
    pub(super) fn from_slabs(
        pos: &[Rect],
        b: &Bundle,
        ends: (Side, Side),
        slabs: &[(bool, f64, f64, f64)],
    ) -> Self {
        let ((sa, sb), m, ra, rb) = (ends, slabs.len(), &pos[b.a], &pos[b.b]);
        let (wa, wb) = (wall_coord(ra, sa), wall_coord(rb, sb));
        let (ca, cb) = (side_center(ra, sa), side_center(rb, sb));
        let mut segs = Vec::with_capacity(m);
        for (i, &(is_horiz, mut lo, mut hi, mid)) in slabs.iter().enumerate() {
            if i == 0 {
                (lo, hi) = iclamp(lo, hi, side_span(ra, sa, b.hs));
            }
            if i + 1 == m {
                (lo, hi) = iclamp(lo, hi, side_span(rb, sb, b.hs));
            }
            if m >= 2 && (i == 1 || i + 2 == m) {
                let (w, sd) = if i == 1 { (wa, sa) } else { (wb, sb) };
                let g = 6.0_f64.min((hi - lo) * 0.25);
                if matches!(sd, Side::B | Side::R) && w + g <= hi {
                    lo = lo.max(w + g);
                } else if !matches!(sd, Side::B | Side::R) && w - g >= lo {
                    hi = hi.min(w - g);
                }
            }
            let r_sd = if is_horiz { Side::L } else { Side::T };
            let tu = match (m, i) {
                (1, _) => (ca + cb) * 0.5,
                (_, 0) => 0.85 * ca + 0.15 * side_center(rb, r_sd),
                _ if i + 1 == m => 0.85 * cb + 0.15 * side_center(ra, r_sd),
                _ => mid,
            };
            let u = tu.clamp(lo, hi);
            let attach = [
                (i == 0).then_some((b.a, sa)),
                (i + 1 == m).then_some((b.b, sb)),
            ];
            segs.push(SegVar {
                is_horiz,
                lo,
                hi,
                span: [0.0; 2],
                u,
                target_u: u,
                attach,
            });
        }
        let mut r = Self {
            a: b.a,
            b: b.b,
            weight: b.edges.len(),
            hs: b.hs,
            sa,
            sb,
            segs,
        };
        update_spans(&mut r, pos);
        r
    }

    pub(super) fn fallback(pos: &[Rect], b: &Bundle) -> Self {
        let (ra, rb) = (&pos[b.a], &pos[b.b]);
        let dn = rb.y >= ra.y;
        let (sa, sb) = if dn {
            (Side::B, Side::T)
        } else {
            (Side::T, Side::B)
        };
        let (ua, ub) = (side_center(ra, sa), side_center(rb, sb));
        let ym = (wall_coord(ra, sa) + wall_coord(rb, sb)) * 0.5;
        let sl = [(false, ua, ua, ua), (true, ym, ym, ym), (false, ub, ub, ub)];
        Self::from_slabs(pos, b, (sa, sb), &sl)
    }

    pub(super) fn poly(&self, pos: &[Rect]) -> Vec<Pt> {
        let wp = |r: &Rect, sd: Side, u| {
            let p = |x, y| Pt { x, y };
            [p(u, r.y), p(u, r.y + r.h), p(r.x, u), p(r.x + r.w, u)][sd as usize]
        };
        let mut raw = vec![wp(&pos[self.a], self.sa, self.segs[0].u)];
        for w in self.segs.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let p = if a.is_horiz { (b.u, a.u) } else { (a.u, b.u) };
            raw.push(Pt { x: p.0, y: p.1 });
        }
        raw.push(wp(&pos[self.b], self.sb, self.segs.last().unwrap().u));
        let mut clean: Vec<Pt> = Vec::with_capacity(raw.len());
        for &q in &raw {
            if let Some(l) = clean.last()
                && (q.x - l.x).abs() + (q.y - l.y).abs() < 0.5
            {
                continue;
            }
            while let [.., a, b] = *clean.as_slice() {
                let cx = (q.x - a.x).abs() < 0.5 && (b.y - a.y) * (q.y - b.y) >= 0.0;
                let cy = (q.y - a.y).abs() < 0.5 && (b.x - a.x) * (q.x - b.x) >= 0.0;
                if !cx && !cy {
                    break;
                }
                clean.pop();
            }
            clean.push(q);
        }
        if clean.len() < 2 {
            return vec![raw[0], *raw.last().unwrap()];
        }
        clean
    }
}

pub(super) fn inset(lo: f64, hi: f64, d: f64) -> (f64, f64) {
    let (a, b, m) = (lo + d, hi - d, (lo + hi) * 0.5);
    if a <= b { (a, b) } else { (m, m) }
}

fn iclamp(lo: f64, hi: f64, (s0, s1): (f64, f64)) -> (f64, f64) {
    let (a, b, m) = (lo.max(s0), hi.min(s1), ((lo + hi) * 0.5).clamp(s0, s1));
    if a <= b { (a, b) } else { (m, m) }
}

pub(super) fn update_spans(r: &mut SegRoute, pos: &[Rect]) {
    let (wa, wb) = (wall_coord(&pos[r.a], r.sa), wall_coord(&pos[r.b], r.sb));
    let us: Vec<f64> = r.segs.iter().map(|s| s.u).collect();
    for (k, seg) in r.segs.iter_mut().enumerate() {
        let p0 = if k == 0 { wa } else { us[k - 1] };
        let p1 = us.get(k + 1).copied().unwrap_or(wb);
        seg.span = [p0.min(p1), p0.max(p1)];
    }
}

/// Optimize routes for all non-self edges on fixed brick rectangles `pos`.
pub(crate) fn optimize_routes(
    pos: &[Rect],
    edges: &[(usize, usize)],
    ids: &[String],
    fast: bool,
) -> RouteOut {
    let bs = group_bundles(pos, edges, ids);
    if bs.is_empty() {
        return RouteOut {
            routes: vec![None; edges.len()],
            ..RouteOut::default()
        };
    }
    let mut active = vec![false; pos.len()];
    for b in &bs {
        (active[b.a], active[b.b]) = (true, true);
    }
    let grid = hanan::HananGrid::build(pos, &active);
    let mut seg_routes = pathfinder::route(&grid, pos, &bs, fast);
    solve::solve_and_eval(pos, &bs, &mut seg_routes, edges.len())
}

pub(super) fn side_span(r: &Rect, side: Side, hs: f64) -> (f64, f64) {
    let tb = matches!(side, Side::T | Side::B);
    let (base, len) = if tb { (r.x, r.w) } else { (r.y, r.h) };
    inset(base, base + len, PAD.min(len * 0.25) + hs)
}

pub(super) fn side_center(r: &Rect, side: Side) -> f64 {
    let tb = matches!(side, Side::T | Side::B);
    if tb { r.x + r.w * 0.5 } else { r.y + r.h * 0.5 }
}

pub(super) fn wall_coord(r: &Rect, side: Side) -> f64 {
    [r.y, r.y + r.h, r.x, r.x + r.w][side as usize]
}

fn group_bundles(pos: &[Rect], edges: &[(usize, usize)], ids: &[String]) -> Vec<Bundle> {
    let mut map: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (ei, &(a, b)) in edges.iter().enumerate() {
        if a != b && a < pos.len() && b < pos.len() {
            map.entry((a, b)).or_default().push(ei);
        }
    }
    let id_of = |i: usize| ids.get(i).map(String::as_str).unwrap_or("");
    let mut out: Vec<Bundle> = map
        .into_iter()
        .map(|((a, b), mut edges)| {
            edges.sort_by(|&x, &y| id_of(x).cmp(id_of(y)).then(x.cmp(&y)));
            let hs = edges.len().saturating_sub(1) as f64 * 0.5 * BUNDLE_PITCH;
            Bundle { a, b, edges, hs }
        })
        .collect();
    let ctr = |i: usize| (pos[i].x + pos[i].w * 0.5, pos[i].y + pos[i].h * 0.5);
    let d = |b: &Bundle| (ctr(b.a).0 - ctr(b.b).0).abs() + (ctr(b.a).1 - ctr(b.b).1).abs();
    out.sort_by(|x, y| d(x).total_cmp(&d(y)).then((x.a, x.b).cmp(&(y.a, y.b))));
    out
}
