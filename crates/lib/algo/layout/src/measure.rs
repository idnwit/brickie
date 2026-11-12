//! Layout quality on sampled paths (`mkItem`, `measureItems`, `scoreLayout`): crossings between
//! edges, edge-through-box hits and direction changes. Used by the tests.

use super::geometry::bbox;
use super::path::route_path;
use super::*;

/// One edge's sampled path for [`measure_items`] (`mkItem`).
#[derive(Debug, Clone)]
pub struct Item {
    pub a: usize,
    pub b: usize,
    pub pts: Vec<Pt>,
    pub bb: [f64; 4],
}

/// `mkItem`.
pub fn mk_item(a: usize, b: usize, pts: Vec<Pt>) -> Item {
    Item {
        a,
        b,
        bb: bbox(&pts),
        pts,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Metrics {
    pub edges: usize,
    pub crossings: usize,
    pub box_hits: usize,
    pub bends: usize,
}

/// `measureItems`: crossings between edges, edge-through-box hits, direction changes.
pub fn measure_items(es: &[Item], nodes: &[Rect]) -> Metrics {
    let inb = |n: &Rect, q: &Pt, m: f64| {
        q.x > n.x - m && q.x < n.x + n.w + m && q.y > n.y - m && q.y < n.y + n.h + m
    };
    let cr = |p: Pt, q: Pt, r: Pt, s: Pt| {
        let d = |a: Pt, b: Pt, c: Pt| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        d(p, q, r) * d(p, q, s) < 0.0 && d(r, s, p) * d(r, s, q) < 0.0
    };
    let (mut crossings, mut box_hits, mut bends) = (0, 0, 0);
    for i in 0..es.len() {
        for j in i + 1..es.len() {
            let (a, b) = (&es[i], &es[j]);
            if a.bb[1] < b.bb[0] || b.bb[1] < a.bb[0] || a.bb[3] < b.bb[2] || b.bb[3] < a.bb[2] {
                continue;
            }
            let sh: Vec<usize> = [a.a, a.b]
                .into_iter()
                .filter(|x| *x == b.a || *x == b.b)
                .collect();
            let mut last = -9i64;
            for x in 0..a.pts.len().saturating_sub(1) {
                for y in 0..b.pts.len().saturating_sub(1) {
                    if !cr(a.pts[x], a.pts[x + 1], b.pts[y], b.pts[y + 1]) {
                        continue;
                    }
                    if sh.iter().any(|&n| inb(&nodes[n], &a.pts[x], 10.0)) || x as i64 - last <= 2 {
                        continue;
                    }
                    last = x as i64;
                    crossings += 1;
                }
            }
        }
    }
    for a in es {
        for (i, n) in nodes.iter().enumerate() {
            if i == a.a || i == a.b {
                continue;
            }
            if a.bb[1] < n.x || a.bb[0] > n.x + n.w || a.bb[3] < n.y || a.bb[2] > n.y + n.h {
                continue;
            }
            if a.pts.iter().any(|q| inb(n, q, -2.0)) {
                box_hits += 1;
            }
        }
    }
    for a in es {
        let (mut sx, mut sy) = (0.0f64, 0.0f64);
        for w in a.pts.windows(2) {
            let (dx, dy) = (w[1].x - w[0].x, w[1].y - w[0].y);
            let gx = if dx.abs() < 0.4 { 0.0 } else { dx.signum() };
            let gy = if dy.abs() < 0.4 { 0.0 } else { dy.signum() };
            if gx != 0.0 {
                if sx != 0.0 && gx != sx {
                    bends += 1;
                }
                sx = gx;
            }
            if gy != 0.0 {
                if sy != 0.0 && gy != sy {
                    bends += 1;
                }
                sy = gy;
            }
        }
    }
    Metrics {
        edges: es.len(),
        crossings,
        box_hits,
        bends,
    }
}

/// `scoreLayout` plus detail: metrics of the edges in `which` (edge indices) on layout `l`.
pub fn score_layout(l: &Layout, which: &[usize]) -> Metrics {
    let items: Vec<Item> = which
        .iter()
        .filter_map(|&e| {
            let r = route_path(l, e, &l.pos)?;
            let (a, b) = l.edges[e];
            Some(mk_item(a, b, r.sample(6.0)))
        })
        .collect();
    measure_items(&items, &l.pos)
}
