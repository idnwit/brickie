//! Segments-as-Variables compaction (solve_axis), energy evaluation, and bundle expansion.

use super::{
    BUNDLE_PITCH, Bundle, BundleStat, Energy, RouteOut, SegRoute, SegVar, TRACK_GAP, side_center,
    update_spans, wall_coord,
};
use crate::geometry::{CORNER_R, bz_port};
use crate::placement::pava;
use crate::{End, Port, Pt, Rect, Route, Side};
use std::collections::HashMap;

pub(super) fn solve_and_eval(
    pos: &[Rect],
    bs: &[Bundle],
    rs: &mut [SegRoute],
    n_e: usize,
) -> RouteOut {
    for is_h in [false, true, false, true] {
        solve_axis(rs, pos, is_h);
        rs.iter_mut().for_each(|r| update_spans(r, pos));
    }
    expand(pos, bs, rs, n_e)
}

fn solve_axis(rs: &mut [SegRoute], pos: &[Rect], is_h: bool) {
    for mut ord in components(rs, is_h) {
        if ord.len() == 1 {
            let s = &mut rs[ord[0].0].segs[ord[0].1];
            s.u = s.target_u.clamp(s.lo, s.hi);
            continue;
        }
        let tu = |x: (usize, usize)| rs[x.0].segs[x.1].target_u;
        ord.sort_by(|&a, &b| tu(a).total_cmp(&tu(b)).then(a.cmp(&b)));
        two_opt(&mut ord, rs, pos);
        for (&(ri, si), u) in ord.iter().zip(project(&ord, rs)) {
            rs[ri].segs[si].u = u;
        }
    }
}

fn components(rs: &[SegRoute], is_h: bool) -> Vec<Vec<(usize, usize)>> {
    let mut items = Vec::new();
    for (ri, r) in rs.iter().enumerate() {
        let e = (0..r.segs.len())
            .filter(|&si| r.segs[si].is_horiz == is_h)
            .map(|si| (ri, si));
        items.extend(e);
    }
    let conflict = |a: &SegVar, b: &SegVar| {
        (0..2).any(|i| a.attach[i].is_some() && b.attach.contains(&a.attach[i]))
            || (a.lo.max(b.lo) <= a.hi.min(b.hi) + TRACK_GAP
                && a.span[0].max(b.span[0]) <= a.span[1].min(b.span[1]) + 1.0)
    };
    let (n, mut seen, mut comps) = (items.len(), vec![false; items.len()], Vec::new());
    for s0 in 0..n {
        if seen[s0] {
            continue;
        }
        seen[s0] = true;
        let (mut comp, mut hd) = (vec![items[s0]], 0);
        while hd < comp.len() {
            let (ri, si) = comp[hd];
            hd += 1;
            for (nxt, &(rj, sj)) in items.iter().enumerate() {
                if !seen[nxt] && conflict(&rs[ri].segs[si], &rs[rj].segs[sj]) {
                    seen[nxt] = true;
                    comp.push((rj, sj));
                }
            }
        }
        comps.push(comp);
    }
    comps
}

fn project(ord: &[(usize, usize)], rs: &[SegRoute]) -> Vec<f64> {
    let (m, mut cum, mut scale) = (ord.len(), vec![0.0; ord.len()], 1.0_f64);
    let seg = |i: usize| &rs[ord[i].0].segs[ord[i].1];
    for i in 1..m {
        cum[i] = cum[i - 1] + rs[ord[i - 1].0].hs + rs[ord[i].0].hs + TRACK_GAP;
    }
    for p in 0..m {
        for q in (p + 1)..m {
            if cum[q] - cum[p] > 1e-6 {
                scale = scale.min((seg(q).hi - seg(p).lo).max(0.0) / (cum[q] - cum[p]));
            }
        }
    }
    let off: Vec<f64> = cum.iter().map(|&c| c * scale).collect();
    let t: Vec<f64> = (0..m).map(|i| seg(i).target_u - off[i]).collect();
    let mut y = pava(&t, &vec![1.0; m]);
    let cl = |i: usize, v: f64| v.clamp(seg(i).lo - off[i], seg(i).hi - off[i]);
    y[0] = cl(0, y[0]);
    for i in 1..m {
        y[i] = cl(i, y[i].max(y[i - 1]));
    }
    for i in (0..m - 1).rev() {
        y[i] = cl(i, y[i].min(y[i + 1]));
    }
    (0..m)
        .map(|i| (y[i] + off[i]).clamp(seg(i).lo, seg(i).hi))
        .collect()
}

fn two_opt(ord: &mut [(usize, usize)], rs: &[SegRoute], pos: &[Rect]) {
    let (m, span) = (ord.len(), if ord.len() <= 20 { ord.len() } else { 8 });
    let (mut best, mut imp, mut pass) = (order_cost(ord, rs, pos), true, 0);
    while imp && pass < 2 {
        (imp, pass) = (false, pass + 1);
        for i in 0..m {
            for j in (i + 1)..m.min(i + 1 + span) {
                ord.swap(i, j);
                let c = order_cost(ord, rs, pos);
                if c + 1e-6 < best {
                    (best, imp) = (c, true);
                } else {
                    ord.swap(i, j);
                }
            }
        }
    }
}

fn order_cost(ord: &[(usize, usize)], rs: &[SegRoute], pos: &[Rect]) -> f64 {
    let us = project(ord, rs);
    let mut legs = Vec::with_capacity(ord.len());
    for (&(ri, si), &u) in ord.iter().zip(&us) {
        legs.push(local_legs(&rs[ri], si, u, pos));
    }
    let (mut inv, mut cross, mut dev) = (0.0, 0.0, 0.0);
    for (i, &(ri, si)) in ord.iter().enumerate() {
        dev += (us[i] - rs[ri].segs[si].target_u).abs() * 0.01;
        for j in (i + 1)..ord.len() {
            let (rj, sj) = ord[j];
            inv += (rs[ri].segs[si].lo - rs[rj].segs[sj].hi).max(0.0);
            cross +=
                cross_polys(&legs[i], &legs[j]) as f64 * (rs[ri].weight * rs[rj].weight) as f64;
        }
    }
    inv * 10_000.0 + cross * 450.0 + dev
}

fn local_legs(r: &SegRoute, si: usize, u: f64, pos: &[Rect]) -> [Pt; 4] {
    let (wa, wb, n) = (
        wall_coord(&pos[r.a], r.sa),
        wall_coord(&pos[r.b], r.sb),
        r.segs.len(),
    );
    let p0 = if si == 0 { wa } else { r.segs[si - 1].u };
    let u0 = if si == 0 { u } else { wa };
    let f0 = si.checked_sub(2).map_or(u0, |k| r.segs[k].u);
    let p1 = r.segs.get(si + 1).map_or(wb, |s| s.u);
    let u1 = if si + 1 == n { u } else { wb };
    let f1 = r.segs.get(si + 2).map_or(u1, |s| s.u);
    let mut ps = [(p0, f0), (p0, u), (p1, u), (p1, f1)].map(|(x, y)| Pt { x, y });
    if !r.segs[si].is_horiz {
        ps.iter_mut().for_each(|p| (p.x, p.y) = (p.y, p.x));
    }
    ps
}

fn cross_polys(a: &[Pt], b: &[Pt]) -> usize {
    let hz = |p: &[Pt]| (p[0].y - p[1].y).abs() < 0.25;
    let ins = |v: f64, p: f64, q: f64| v > p.min(q) + 0.5 && v < p.max(q) - 0.5;
    let mut n = 0;
    for s in a.windows(2) {
        for t in b.windows(2) {
            if hz(s) == hz(t) {
                continue;
            }
            let (h, v) = if hz(s) { (s, t) } else { (t, s) };
            n += usize::from(ins(v[0].x, h[0].x, h[1].x) && ins(h[0].y, v[0].y, v[1].y));
        }
    }
    n
}

fn eval_energy(pos: &[Rect], rs: &[SegRoute]) -> (Energy, Vec<BundleStat>, Vec<Vec<Pt>>) {
    let polys: Vec<Vec<Pt>> = rs.iter().map(|r| r.poly(pos)).collect();
    let (mut bc, mut crossings) = (vec![0usize; rs.len()], 0usize);
    for i in 0..rs.len() {
        for j in (i + 1)..rs.len() {
            let c = cross_polys(&polys[i], &polys[j]);
            crossings += c * rs[i].weight * rs[j].weight;
            bc[i] += c;
            bc[j] += c;
        }
    }
    let (mut hard, mut bends, mut length, mut pol, mut aln) = (0.0, 0usize, 0.0, 0.0, 0.0);
    let mut stats = Vec::with_capacity(rs.len());
    for (i, r) in rs.iter().enumerate() {
        let (a, b, weight, nb, mut l) = (r.a, r.b, r.weight, polys[i].len().saturating_sub(2), 0.0);
        for w in polys[i].windows(2) {
            let (x0, x1) = (w[0].x.min(w[1].x), w[0].x.max(w[1].x));
            let (y0, y1) = (w[0].y.min(w[1].y), w[0].y.max(w[1].y));
            l += (x1 - x0) + (y1 - y0);
            for (j, q) in pos.iter().enumerate() {
                let [qx0, qx1, qy0, qy1] = q.bb();
                let hit = x1 > qx0 + 0.5 && x0 < qx1 - 0.5 && y1 > qy0 + 0.5 && y0 < qy1 - 0.5;
                if j != a && j != b && hit {
                    hard += 1.0;
                }
            }
        }
        (bends, length) = (bends + nb * weight, length + l * weight as f64);
        let (ra, rb, pa, pb) = (&pos[a], &pos[b], r.segs[0].u, r.segs.last().unwrap().u);
        if (rb.y >= ra.y + ra.h - 0.5 && (r.sa == Side::T || r.sb == Side::B))
            || (ra.y >= rb.y + rb.h - 0.5 && (r.sa == Side::B || r.sb == Side::T))
        {
            pol += 450.0;
        }
        aln += 0.1 * ((pa - side_center(ra, r.sa)).abs() + (pb - side_center(rb, r.sb)).abs());
        let tb = matches!(r.sa, Side::T | Side::B) && matches!(r.sb, Side::T | Side::B);
        stats.push(BundleStat {
            a,
            b,
            weight,
            bends: nb,
            crossings: bc[i],
            v_align: tb.then_some((pa, pb)),
        });
    }
    let total = 1e6 * hard + 450.0 * crossings as f64 + 320.0 * bends as f64 + pol + length + aln;
    let energy = Energy {
        hard,
        crossings,
        bends,
        length,
        total,
    };
    (energy, stats, polys)
}

fn expand(pos: &[Rect], bs: &[Bundle], rs: &[SegRoute], n: usize) -> RouteOut {
    let (energy, stats, polys) = eval_energy(pos, rs);
    let (mut routes, mut po) = (vec![None; n], HashMap::new());
    for (ri, r) in rs.iter().enumerate() {
        let (b, center, k) = (&bs[ri], &polys[ri], bs[ri].edges.len());
        let (mut pitch, span) = (BUNDLE_PITCH, k.saturating_sub(1) as f64);
        let ns: Vec<_> = center
            .windows(2)
            .map(|w| {
                let (dx, dy) = (w[1].x - w[0].x, w[1].y - w[0].y);
                let l = dx.hypot(dy).max(1e-6);
                (-dy / l, dx / l)
            })
            .collect();
        if k > 1 {
            let room = (pos[b.a].side_len(r.sa).min(pos[b.b].side_len(r.sb)) - 4.0).max(2.0);
            pitch = pitch.min(room / span);
            for i in 1..center.len().saturating_sub(2) {
                let len = (center[i + 1].x - center[i].x).hypot(center[i + 1].y - center[i].y);
                if len < span * pitch + 2.0
                    && ns[i - 1].0 * ns[i + 1].0 + ns[i - 1].1 * ns[i + 1].1 < -0.5
                {
                    pitch = pitch.min(((len - 2.0) / span).max(1.0));
                }
            }
        }
        for (idx, &e) in b.edges.iter().enumerate() {
            let (d, mut pts) = ((idx as f64 - 0.5 * span) * pitch, center.clone());
            if d.abs() >= 1e-6 && center.len() >= 2 {
                for (i, pt) in pts.iter_mut().enumerate() {
                    let n0 = if i > 0 { ns[i - 1] } else { (0.0, 0.0) };
                    let n1 = ns.get(i).copied().unwrap_or((0.0, 0.0));
                    (pt.x, pt.y) = (pt.x + d * (n0.0 + n1.0), pt.y + d * (n0.1 + n1.1));
                }
            }
            for (bx, sd, pi, end) in [(b.a, r.sa, 0, End::A), (b.b, r.sb, pts.len() - 1, End::B)] {
                let (q, p) = (&pos[bx], pts[pi]);
                let lr = matches!(sd, Side::L | Side::R);
                let off = (if lr { p.y } else { p.x }) - side_center(q, sd);
                pts[pi] = bz_port(q, sd, off);
                po.insert((e, end), Port { side: sd, off });
            }
            routes[e] = Some(Route::orthogonal(&pts, CORNER_R));
        }
    }
    RouteOut {
        routes,
        po,
        energy,
        stats,
    }
}
