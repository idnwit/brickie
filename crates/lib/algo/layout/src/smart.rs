//! `smartRoute`: every edge starts with the guaranteed box-free layered route (dummy chain or arc)
//! and may switch to one direct cubic between any pair of box sides. A greedy pass (short edges
//! first, then a second pass against all others) picks per edge the candidate with fewest box
//! hits, then fewest crossings with the edges chosen so far, then side preference and length.
//! Ports on one side are then spread evenly, sorted by where the edge goes; a direct route that
//! hits a box after spreading falls back to its layered route.

use super::geometry::{bbox, bz_ctl, bz_port, bz_sample, poly_cross, poly_len, poly_near};
use super::path::{route_path, route_with};
use super::*;

struct Cand {
    old: bool,
    sa: Side,
    sb: Side,
    pen: f64,
    pts: Vec<Pt>,
    boxh: usize,
    len: f64,
    bb: [f64; 4],
    cp: Vec<Pt>,
}

fn coarse(p: &[Pt]) -> Vec<Pt> {
    p.iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0 || *i == p.len() - 1)
        .map(|(_, q)| *q)
        .collect()
}

fn side_len(n: &Rect, sd: Side) -> f64 {
    if matches!(sd, Side::T | Side::B) {
        n.w
    } else {
        n.h
    }
}

fn cap(n: &Rect, sd: Side) -> usize {
    (((side_len(n, sd) - 2.0 * PAD) / PORT).floor() as i64 + 1).max(1) as usize
}

fn gap_to(a: &Rect, sd: Side, b: &Rect) -> f64 {
    match sd {
        Side::R => b.x - (a.x + a.w),
        Side::L => a.x - (b.x + b.w),
        Side::B => b.y - (a.y + a.h),
        Side::T => a.y - (b.y + b.h),
    }
}

/// `smartRoute`: pick a direct cubic or the layered route per edge, then spread the ports.
pub fn smart_route(l: &mut Layout, ids: &[String]) {
    let pos = l.pos.clone();
    let edges = l.edges.clone();
    let eids: Vec<usize> = (0..edges.len())
        .filter(|&e| l.kinds[e].is_some() && edges[e].0 != edges[e].1)
        .collect();
    let old_kind = l.kinds.clone();
    let old_po = l.po.clone();
    let hits = |pts: &[Pt], e: usize| {
        let mut h = 0;
        for (i, n) in pos.iter().enumerate() {
            let own = i == edges[e].0 || i == edges[e].1;
            let m = if own { -1.0 } else { 6.0 };
            let (lo, hi) = if own {
                (3, pts.len().saturating_sub(3))
            } else {
                (0, pts.len())
            };
            for q in pts.iter().take(hi).skip(lo) {
                if q.x > n.x - m && q.x < n.x + n.w + m && q.y > n.y - m && q.y < n.y + n.h + m {
                    h += 1;
                    break;
                }
            }
        }
        h
    };
    let mut cands: Vec<Vec<Cand>> = (0..edges.len()).map(|_| Vec::new()).collect();
    for &e in &eids {
        let (ia, ib) = edges[e];
        let (a, b) = (&pos[ia], &pos[ib]);
        let ok = old_kind[e].clone().unwrap_or(Kind::Bez);
        let old_pts = route_with(&ok, &old_po, &l.dpos, e, edges[e], &pos)
            .map(|r| r.sample(7.0))
            .unwrap_or_default();
        let sa0 = old_po.get(&(e, End::A)).map_or(Side::T, |p| p.side);
        let sb0 = old_po.get(&(e, End::B)).map_or(Side::T, |p| p.side);
        let pen = match ok {
            Kind::Arc { .. } => 45.0,
            _ => 12.0,
        };
        let mut list = vec![Cand {
            old: true,
            sa: sa0,
            sb: sb0,
            pen,
            pts: old_pts,
            boxh: 0,
            len: 0.0,
            bb: [0.0; 4],
            cp: vec![],
        }];
        for sa in Side::ALL {
            for sb in Side::ALL {
                let (ga, gb) = (gap_to(a, sa, b), gap_to(b, sb, a));
                if ga < 14.0 || gb < 14.0 {
                    continue; // both ends must face each other with room for the tangent
                }
                let dx = (a.x.max(b.x) - (a.x + a.w).min(b.x + b.w)).max(0.0);
                let dy = (a.y.max(b.y) - (a.y + a.h).min(b.y + b.h)).max(0.0);
                let pen = if sa.horizontal() && sb.horizontal() {
                    if dy > dx { 40.0 } else { 0.0 }
                } else if !sa.horizontal() && !sb.horizontal() {
                    if dx > dy * 1.6 { 25.0 } else { 0.0 }
                } else if dx.min(dy) < 30.0 {
                    30.0
                } else {
                    8.0
                };
                let c = bz_ctl(bz_port(a, sa, 0.0), sa, bz_port(b, sb, 0.0), sb);
                list.push(Cand {
                    old: false,
                    sa,
                    sb,
                    pen,
                    pts: bz_sample(&c, 7.0),
                    boxh: 0,
                    len: 0.0,
                    bb: [0.0; 4],
                    cp: vec![],
                });
            }
        }
        for c in list.iter_mut() {
            c.boxh = hits(&c.pts, e);
            c.len = poly_len(&c.pts);
            c.bb = bbox(&c.pts);
            c.cp = coarse(&c.pts);
        }
        cands[e] = list.into_iter().filter(|c| c.old || c.boxh == 0).collect();
    }
    let mut cnt: HashMap<(usize, Side), usize> = HashMap::new();
    let mut choice: Vec<Option<usize>> = vec![None; edges.len()];
    let cost = |e: usize, c: &Cand, choice: &[Option<usize>], cands: &[Vec<Cand>]| {
        let (mut x, mut near) = (0.0, 0.0);
        for &o in &eids {
            let Some(ci) = choice[o] else { continue };
            if o == e {
                continue;
            }
            let oc = &cands[o][ci];
            if oc.bb[1] < c.bb[0] - 8.0
                || c.bb[1] < oc.bb[0] - 8.0
                || oc.bb[3] < c.bb[2] - 8.0
                || c.bb[3] < oc.bb[2] - 8.0
            {
                continue;
            }
            let (f, g) = (edges[o], edges[e]);
            let shared = f.0 == g.0 || f.0 == g.1 || f.1 == g.0 || f.1 == g.1;
            x += poly_cross(&c.cp, &oc.cp) as f64;
            if !shared {
                near += poly_near(&c.cp, &oc.pts, 8.0);
            }
        }
        c.boxh as f64 * 1000.0 + x * 30.0 + near * 0.6 + c.pen + c.len * 0.02
    };
    let mut order = eids.clone();
    let min_len = |e: usize| cands[e].iter().map(|c| c.len).fold(f64::INFINITY, f64::min);
    order.sort_by(|&p, &q| min_len(p).total_cmp(&min_len(q)).then(ids[p].cmp(&ids[q])));
    let take = |cnt: &mut HashMap<(usize, Side), usize>, e: usize, c: &Cand, add: bool| {
        for k in [(edges[e].0, c.sa), (edges[e].1, c.sb)] {
            let v = cnt.entry(k).or_insert(0);
            *v = if add { *v + 1 } else { v.saturating_sub(1) };
        }
    };
    for _pass in 0..2 {
        for &e in &order {
            if let Some(ci) = choice[e].take() {
                take(&mut cnt, e, &cands[e][ci], false);
            }
            let (mut best, mut bc) = (0, f64::INFINITY);
            for (ci, c) in cands[e].iter().enumerate() {
                if !c.old
                    && (cnt.get(&(edges[e].0, c.sa)).copied().unwrap_or(0)
                        >= cap(&pos[edges[e].0], c.sa)
                        || cnt.get(&(edges[e].1, c.sb)).copied().unwrap_or(0)
                            >= cap(&pos[edges[e].1], c.sb))
                {
                    continue;
                }
                let v = cost(e, c, &choice, &cands);
                if v < bc {
                    bc = v;
                    best = ci;
                }
            }
            choice[e] = Some(best);
            take(&mut cnt, e, &cands[e][best], true);
        }
    }
    // Spread ports per (brick, side), sorted by the direction the edge leaves in.
    let assign = |choice: &[Option<usize>], cands: &[Vec<Cand>]| {
        struct Pl {
            key: (usize, End),
            q: Pt,
            far: Pt,
        }
        let mut pl: HashMap<(usize, Side), Vec<Pl>> = HashMap::new();
        for &e in &eids {
            let c = &cands[e][choice[e].unwrap_or(0)];
            let k = c.pts.len().saturating_sub(1).min(REFK);
            let (first, last) = (c.pts[0], c.pts[c.pts.len() - 1]);
            pl.entry((edges[e].0, c.sa)).or_default().push(Pl {
                key: (e, End::A),
                q: c.pts[k],
                far: last,
            });
            pl.entry((edges[e].1, c.sb)).or_default().push(Pl {
                key: (e, End::B),
                q: c.pts[c.pts.len() - 1 - k],
                far: first,
            });
        }
        let mut po: HashMap<(usize, End), Port> = HashMap::new();
        for ((n, sd), mut list) in pl {
            let hz = matches!(sd, Side::T | Side::B);
            list.sort_by(|p, q| {
                let key = |x: &Pl| (ids[x.key.0].clone(), x.key.1 == End::B);
                if hz {
                    p.q.x.total_cmp(&q.q.x).then(p.far.x.total_cmp(&q.far.x))
                } else {
                    p.q.y
                        .total_cmp(&q.q.y)
                        .then(p.far.y.total_cmp(&q.far.y))
                        .then(key(p).cmp(&key(q)))
                }
            });
            let len = side_len(&pos[n], sd) - 2.0 * PAD;
            let n_l = list.len() as f64;
            let st = if n_l > 1.0 {
                (len / (n_l - 1.0)).min(26.0)
            } else {
                0.0
            };
            for (i, p) in list.iter().enumerate() {
                po.insert(
                    p.key,
                    Port {
                        side: sd,
                        off: (i as f64 - (n_l - 1.0) / 2.0) * st,
                    },
                );
            }
        }
        po
    };
    let mut po;
    for _round in 0..8 {
        po = assign(&choice, &cands);
        let mut changed = false;
        for &e in &eids {
            let ci = choice[e].unwrap_or(0);
            if cands[e][ci].old {
                continue;
            }
            let (sa, sb) = (cands[e][ci].sa, cands[e][ci].sb);
            let (pa, pb) = (po[&(e, End::A)].off, po[&(e, End::B)].off);
            let pts = bz_sample(
                &bz_ctl(
                    bz_port(&pos[edges[e].0], sa, pa),
                    sa,
                    bz_port(&pos[edges[e].1], sb, pb),
                    sb,
                ),
                4.0,
            );
            if hits(&pts, e) > 0 {
                choice[e] = Some(0);
                changed = true;
            } else {
                let c = &mut cands[e][ci];
                c.bb = bbox(&pts);
                c.cp = coarse(&pts);
                c.pts = pts;
            }
        }
        if !changed {
            break;
        }
    }
    po = assign(&choice, &cands);
    for &e in &eids {
        let old = cands[e][choice[e].unwrap_or(0)].old;
        l.kinds[e] = if old {
            old_kind[e].clone()
        } else {
            Some(Kind::Bez)
        };
    }
    l.po = po;
    for e in 0..edges.len() {
        l.routes[e] = route_path(l, e, &pos);
    }
}
