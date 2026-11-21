//! Routes of the frame edges ("outside -> frame"): one cubic from the outside neighbour to the
//! nearest frame wall. Edges of the same neighbour are spread a little along both ends.

use crate::layout::{Pt, Rect, Route};
use brickie_web_api::GraphView;

/// Space between two frame edges of the same neighbour.
const SLOT: f64 = 16.0;

fn ortho_v(p: Pt, q: Pt, off: f64) -> Vec<Pt> {
    if (p.x - q.x).abs() < 1.0 {
        return vec![p, q];
    }
    let my = (p.y + q.y) / 2.0 + (off * 0.25).clamp(-14.0, 14.0);
    vec![p, Pt { x: p.x, y: my }, Pt { x: q.x, y: my }, q]
}

fn ortho_h(p: Pt, q: Pt) -> Vec<Pt> {
    if (p.y - q.y).abs() < 1.0 {
        return vec![p, q];
    }
    let mx = (p.x + q.x) / 2.0;
    vec![p, Pt { x: mx, y: p.y }, Pt { x: mx, y: q.y }, q]
}

/// One softly rounded orthogonal route between outside neighbour `n` and the nearest wall of
/// frame `f` (`into = true` goes neighbour -> frame, `false` goes frame -> neighbour), offset by
/// `off` along the wall.
pub fn wall_route(n: &Rect, f: &Rect, into: bool, off: f64) -> Route {
    let (cx, cy) = (n.x + n.w / 2.0, n.y + n.h / 2.0);
    let clamp = |v: f64, lo: f64, len: f64| v.clamp(lo + SLOT, (lo + len - SLOT).max(lo + SLOT));
    let mut pts = if n.y + n.h <= f.y {
        let px = cx + off;
        let qx = clamp(px, f.x, f.w);
        ortho_v(
            Pt {
                x: px,
                y: n.y + n.h,
            },
            Pt { x: qx, y: f.y },
            off,
        )
    } else if n.y >= f.y + f.h {
        let px = cx + off;
        let qx = clamp(px, f.x, f.w);
        ortho_v(
            Pt { x: px, y: n.y },
            Pt {
                x: qx,
                y: f.y + f.h,
            },
            off,
        )
    } else if n.x + n.w <= f.x {
        let py = cy + off;
        let qy = clamp(py, f.y, f.h);
        ortho_h(
            Pt {
                x: n.x + n.w,
                y: py,
            },
            Pt { x: f.x, y: qy },
        )
    } else {
        let py = cy + off;
        let qy = clamp(py, f.y, f.h);
        ortho_h(
            Pt { x: n.x, y: py },
            Pt {
                x: f.x + f.w,
                y: qy,
            },
        )
    };
    if !into {
        pts.reverse();
    }
    Route::orthogonal(&pts, 12.0)
}

/// The route of frame edge `e` for box positions `pos` and the frame `f`.
pub fn frame_route(g: &GraphView, e: usize, pos: &[Rect], f: &Rect) -> Option<Route> {
    let x = &g.edges[e];
    let into = g.bricks[x.from].ghost;
    let ghost = if into { x.from } else { x.to };
    let n = pos.get(ghost)?;
    // Slot of this edge among the frame edges of the same neighbour, centred on 0.
    let mine: Vec<usize> = (0..g.edges.len())
        .filter(|&i| g.edges[i].frame() && (g.edges[i].from == ghost || g.edges[i].to == ghost))
        .collect();
    let k = mine.iter().position(|&i| i == e).unwrap_or(0) as f64;
    let off = (k - (mine.len() as f64 - 1.0) / 2.0) * SLOT;
    Some(wall_route(n, f, into, off))
}
