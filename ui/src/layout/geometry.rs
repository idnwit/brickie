//! Routes as cubic Bezier segments and the geometry helpers of the original: `bzPort`, `bzCtl`,
//! `bzSample`, `polyLen`, `polyCross`, `polyNear`.

use super::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Route {
    pub segs: Vec<[Pt; 4]>,
}

impl Route {
    /// SVG path data (`d` attribute), one decimal.
    pub fn d(&self) -> String {
        let mut s = String::new();
        for (i, c) in self.segs.iter().enumerate() {
            if i == 0 {
                s += &format!("M{:.1},{:.1}", c[0].x, c[0].y);
            }
            s += &format!(
                " C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                c[1].x, c[1].y, c[2].x, c[2].y, c[3].x, c[3].y
            );
        }
        s
    }
    /// Points along the path about `step` apart (replaces `getPointAtLength` sampling).
    pub fn sample(&self, step: f64) -> Vec<Pt> {
        let mut out: Vec<Pt> = Vec::new();
        for c in &self.segs {
            let pts = bz_sample(c, step);
            out.extend(if out.is_empty() { &pts[..] } else { &pts[1..] });
        }
        out
    }
    /// A point near the middle (edge label / mark anchor).
    pub fn mid(&self) -> Pt {
        let p = self.sample(6.0);
        p.get(p.len() / 2).copied().unwrap_or_default()
    }
}

/// `bzPort`.
pub fn bz_port(n: &Rect, sd: Side, off: f64) -> Pt {
    match sd {
        Side::T => Pt {
            x: n.x + n.w / 2.0 + off,
            y: n.y,
        },
        Side::B => Pt {
            x: n.x + n.w / 2.0 + off,
            y: n.y + n.h,
        },
        Side::L => Pt {
            x: n.x,
            y: n.y + n.h / 2.0 + off,
        },
        Side::R => Pt {
            x: n.x + n.w,
            y: n.y + n.h / 2.0 + off,
        },
    }
}

/// `bzCtl`: control points of a cubic with tangents normal to the two sides.
pub fn bz_ctl(p: Pt, sp: Side, q: Pt, sq: Side) -> [Pt; 4] {
    let d = (q.x - p.x).hypot(q.y - p.y);
    let c = (d * 0.42).clamp(22.0, 150.0);
    let (np, nq) = (sp.normal(), sq.normal());
    [
        p,
        Pt {
            x: p.x + np.0 * c,
            y: p.y + np.1 * c,
        },
        Pt {
            x: q.x + nq.0 * c,
            y: q.y + nq.1 * c,
        },
        q,
    ]
}

/// `bzSample`.
pub fn bz_sample(c: &[Pt; 4], step: f64) -> Vec<Pt> {
    let len = (c[3].x - c[0].x).hypot(c[3].y - c[0].y)
        + (c[1].x - c[0].x).hypot(c[1].y - c[0].y)
        + (c[3].x - c[2].x).hypot(c[3].y - c[2].y);
    let n = ((len / step).ceil() as usize).max(12);
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let (a, b, d, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            Pt {
                x: a * c[0].x + b * c[1].x + d * c[2].x + e * c[3].x,
                y: a * c[0].y + b * c[1].y + d * c[2].y + e * c[3].y,
            }
        })
        .collect()
}

/// `polyLen`.
pub fn poly_len(p: &[Pt]) -> f64 {
    p.windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum()
}

/// `polyCross`: proper intersections between two polylines; touching runs count once.
pub fn poly_cross(a: &[Pt], b: &[Pt]) -> usize {
    let (mut c, mut last) = (0usize, -9i64);
    for i in 0..a.len().saturating_sub(1) {
        let (p, q) = (a[i], a[i + 1]);
        let mut hit = false;
        for j in 0..b.len().saturating_sub(1) {
            let (r, s) = (b[j], b[j + 1]);
            let (d1x, d1y, d2x, d2y) = (q.x - p.x, q.y - p.y, s.x - r.x, s.y - r.y);
            let den = d1x * d2y - d1y * d2x;
            if den.abs() < 1e-9 {
                continue;
            }
            let t = ((r.x - p.x) * d2y - (r.y - p.y) * d2x) / den;
            let u = ((r.x - p.x) * d1y - (r.y - p.y) * d1x) / den;
            if (0.0..1.0).contains(&t) && (0.0..1.0).contains(&u) {
                hit = true;
                break;
            }
        }
        if hit {
            if i as i64 - last > 2 {
                c += 1;
            }
            last = i as i64;
        }
    }
    c
}

/// `polyNear`: length of `a` running closer than `dmin` to `b`'s points.
pub fn poly_near(a: &[Pt], b: &[Pt], dmin: f64) -> f64 {
    let d2 = dmin * dmin;
    let mut l = 0.0;
    for q in a {
        if b.iter()
            .any(|r| (r.x - q.x).powi(2) + (r.y - q.y).powi(2) < d2)
        {
            l += 14.0;
        }
    }
    l
}

/// Bounding box `[x0, x1, y0, y1]` of a polyline.
pub fn bbox(pts: &[Pt]) -> [f64; 4] {
    let mut b: [f64; 4] = [1e9, -1e9, 1e9, -1e9];
    for q in pts {
        b[0] = b[0].min(q.x);
        b[1] = b[1].max(q.x);
        b[2] = b[2].min(q.y);
        b[3] = b[3].max(q.y);
    }
    b
}
