//! `routePath`: the path of one edge from its route kind, ports and the (possibly dragged) box
//! positions. Layered routes pass through the dummy nodes of their chain.

use super::geometry::{CORNER_R, Route, bz_port, ortho_direct};
use super::*;

/// `routePath` for one edge: its route kind, ports, channel tracks, row bands and box positions `p`.
#[allow(clippy::too_many_arguments)]
pub fn route_with(
    kind: &Kind,
    po: &HashMap<(usize, End), Port>,
    dpos: &[Option<(f64, f64, f64)>],
    tracks: &HashMap<(usize, usize), f64>,
    rows: &[RowOut],
    e: usize,
    ends: (usize, usize),
    p: &[Rect],
) -> Option<Route> {
    let (a, b) = (p.get(ends.0)?, p.get(ends.1)?);
    let port = |n: &Rect, end: End| {
        let pt = po.get(&(e, end)).copied().unwrap_or(Port {
            side: Side::T,
            off: 0.0,
        });
        bz_port(n, pt.side, pt.off)
    };
    let side = |end: End| po.get(&(e, end)).map_or(Side::T, |x| x.side);
    let track = |hop: usize| tracks.get(&(e, hop)).copied().unwrap_or(0.5);
    match kind {
        Kind::Bez => Some(ortho_direct(
            port(a, End::A),
            side(End::A),
            port(b, End::B),
            side(End::B),
            track(0),
            Some(row_bottom(rows, ends.0, a)),
            Some(row_bottom(rows, ends.1, b)),
        )),
        Kind::Arc { top, level } => {
            let (pa, pb) = (port(a, End::A), port(b, End::B));
            let h = 16.0 + 14.0 * *level as f64;
            let y = if *top {
                pa.y.min(pb.y) - h
            } else {
                row_bottom(rows, ends.0, a)
                    .max(row_bottom(rows, ends.1, b))
                    .max(pa.y.max(pb.y))
                    + h
            };
            Some(Route::orthogonal(
                &[pa, Pt { x: pa.x, y }, Pt { x: pb.x, y }, pb],
                10.0,
            ))
        }
        Kind::Chain {
            chain,
            up_end,
            idx,
            cnt,
        } => {
            let u = p.get(chain[0])?;
            let lo = p.get(chain[chain.len() - 1])?;
            let u_pt = port(u, *up_end);
            let l_pt = port(lo, up_end.other());
            let u_chan_y = if side(*up_end) == Side::B {
                row_bottom(rows, chain[0], u).max(u_pt.y)
            } else {
                u_pt.y
            };
            let lat = (*idx as f64 - (*cnt as f64 - 1.0) / 2.0) * LANE;
            // (x, y, half height; None for the end ports, channel boundary y)
            let mut c0: Vec<(f64, f64, Option<f64>, f64)> = vec![(u_pt.x, u_pt.y, None, u_chan_y)];
            for &k in &chain[1..chain.len() - 1] {
                let (dx, dy, dh) = dpos.get(k).copied().flatten()?;
                c0.push((dx + lat, dy + dh / 2.0, Some(dh / 2.0), dy));
            }
            c0.push((l_pt.x, l_pt.y, None, l_pt.y));
            if *up_end == End::B {
                c0.reverse();
            }
            // Orthogonal waypoints through each dummy corridor, with softly rounded bends in the
            // inter-row channels.
            let mut pts: Vec<Pt> = Vec::new();
            let mut prev_chan_y = 0.0;
            for (i, &(x, y, hh, chan_y)) in c0.iter().enumerate() {
                let (enter, leave, enter_chan_y, leave_chan_y) = match hh {
                    None => (Pt { x, y }, Pt { x, y }, chan_y, chan_y),
                    Some(hh) => {
                        let dir = if c0[i + 1].1 >= c0[i - 1].1 {
                            1.0
                        } else {
                            -1.0
                        };
                        let ey = y - hh * dir;
                        let ly = y + hh * dir;
                        (Pt { x, y: ey }, Pt { x, y: ly }, ey, ly)
                    }
                };
                if let Some(&prev) = pts.last()
                    && (prev.x - enter.x).abs() >= 1.0
                {
                    let (y0, y1) = if (enter_chan_y - prev_chan_y) * (enter.y - prev.y) > 0.0 {
                        (prev_chan_y, enter_chan_y)
                    } else {
                        (prev.y, enter.y)
                    };
                    let (top_y, bot_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
                    let my = top_y + (bot_y - top_y) * track(i);
                    pts.push(Pt { x: prev.x, y: my });
                    pts.push(Pt { x: enter.x, y: my });
                }
                pts.push(enter);
                if hh.is_some() {
                    pts.push(leave);
                }
                prev_chan_y = leave_chan_y;
            }
            Some(Route::orthogonal(&pts, CORNER_R))
        }
    }
}

fn rect_moved(a: &Rect, b: &Rect) -> bool {
    (a.x - b.x).abs() > 0.5 || (a.y - b.y).abs() > 0.5
}

/// Dynamic orthogonal route for a dragged endpoint: picks the cleanest side pair and aligns
/// ports along any shared x/y overlap so moving a brick adapts both shape and port positions.
fn dynamic_route(p: &[Rect], ends: (usize, usize)) -> Option<Route> {
    let (a, b) = (p.get(ends.0)?, p.get(ends.1)?);
    let (sa, sb, pa, pb) = if a.y + a.h + 14.0 <= b.y {
        let lo = (a.x + PAD).max(b.x + PAD);
        let hi = (a.x + a.w - PAD).min(b.x + b.w - PAD);
        let (xa, xb) = if lo <= hi {
            let x = (lo + hi) / 2.0;
            (x, x)
        } else {
            (a.x + a.w / 2.0, b.x + b.w / 2.0)
        };
        (
            Side::B,
            Side::T,
            Pt {
                x: xa,
                y: a.y + a.h,
            },
            Pt { x: xb, y: b.y },
        )
    } else if b.y + b.h + 14.0 <= a.y {
        let lo = (a.x + PAD).max(b.x + PAD);
        let hi = (a.x + a.w - PAD).min(b.x + b.w - PAD);
        let (xa, xb) = if lo <= hi {
            let x = (lo + hi) / 2.0;
            (x, x)
        } else {
            (a.x + a.w / 2.0, b.x + b.w / 2.0)
        };
        (
            Side::T,
            Side::B,
            Pt { x: xa, y: a.y },
            Pt {
                x: xb,
                y: b.y + b.h,
            },
        )
    } else if a.x + a.w + 14.0 <= b.x {
        let lo = (a.y + PAD).max(b.y + PAD);
        let hi = (a.y + a.h - PAD).min(b.y + b.h - PAD);
        let (ya, yb) = if lo <= hi {
            let y = (lo + hi) / 2.0;
            (y, y)
        } else {
            (a.y + a.h / 2.0, b.y + b.h / 2.0)
        };
        (
            Side::R,
            Side::L,
            Pt {
                x: a.x + a.w,
                y: ya,
            },
            Pt { x: b.x, y: yb },
        )
    } else if b.x + b.w + 14.0 <= a.x {
        let lo = (a.y + PAD).max(b.y + PAD);
        let hi = (a.y + a.h - PAD).min(b.y + b.h - PAD);
        let (ya, yb) = if lo <= hi {
            let y = (lo + hi) / 2.0;
            (y, y)
        } else {
            (a.y + a.h / 2.0, b.y + b.h / 2.0)
        };
        (
            Side::L,
            Side::R,
            Pt { x: a.x, y: ya },
            Pt {
                x: b.x + b.w,
                y: yb,
            },
        )
    } else {
        (
            Side::B,
            Side::B,
            bz_port(a, Side::B, 0.0),
            bz_port(b, Side::B, 0.0),
        )
    };
    Some(ortho_direct(pa, sa, pb, sb, 0.5, None, None))
}

/// `routePath(e, L, P)`.
pub fn route_path(l: &Layout, e: usize, p: &[Rect]) -> Option<Route> {
    let ends = l.edges[e];
    let moved = p
        .get(ends.0)
        .zip(l.pos.get(ends.0))
        .is_some_and(|(cur, orig)| rect_moved(cur, orig))
        || p.get(ends.1)
            .zip(l.pos.get(ends.1))
            .is_some_and(|(cur, orig)| rect_moved(cur, orig));
    if moved {
        return dynamic_route(p, ends);
    }
    route_with(
        l.kinds[e].as_ref()?,
        &l.po,
        &l.dpos,
        &l.tracks,
        &l.rows,
        e,
        ends,
        p,
    )
}
