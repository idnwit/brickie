//! Routes of the frame edges ("outside -> frame"): one cubic from the outside neighbour to the
//! nearest frame wall. Edges of the same neighbour are spread a little along both ends.

use crate::layout::{Pt, Rect, Route};
use crate::model::GraphView;

/// Space between two frame edges of the same neighbour.
const SLOT: f64 = 16.0;

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
    let (cx, cy) = (n.x + n.w / 2.0, n.y + n.h / 2.0);
    let clamp = |v: f64, lo: f64, len: f64| v.clamp(lo + SLOT, (lo + len - SLOT).max(lo + SLOT));
    // Ends and the unit direction from the neighbour towards the frame.
    let (p, q, dir) = if n.y + n.h <= f.y {
        let px = cx + off;
        (
            Pt {
                x: px,
                y: n.y + n.h,
            },
            Pt {
                x: clamp(px, f.x, f.w),
                y: f.y,
            },
            (0.0, 1.0),
        )
    } else if n.y >= f.y + f.h {
        let px = cx + off;
        (
            Pt { x: px, y: n.y },
            Pt {
                x: clamp(px, f.x, f.w),
                y: f.y + f.h,
            },
            (0.0, -1.0),
        )
    } else if n.x + n.w <= f.x {
        let py = cy + off;
        (
            Pt {
                x: n.x + n.w,
                y: py,
            },
            Pt {
                x: f.x,
                y: clamp(py, f.y, f.h),
            },
            (1.0, 0.0),
        )
    } else {
        let py = cy + off;
        (
            Pt { x: n.x, y: py },
            Pt {
                x: f.x + f.w,
                y: clamp(py, f.y, f.h),
            },
            (-1.0, 0.0),
        )
    };
    let t = ((q.x - p.x).abs() + (q.y - p.y).abs()).max(20.0) * 0.45;
    let c1 = Pt {
        x: p.x + dir.0 * t,
        y: p.y + dir.1 * t,
    };
    let c2 = Pt {
        x: q.x - dir.0 * t,
        y: q.y - dir.1 * t,
    };
    let seg = if into { [p, c1, c2, q] } else { [q, c2, c1, p] };
    Some(Route { segs: vec![seg] })
}
