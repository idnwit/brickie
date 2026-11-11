//! `routePath`: the path of one edge from its route kind, ports and the (possibly dragged) box
//! positions. Layered routes pass through the dummy nodes of their chain.

use super::geometry::{Route, bz_ctl, bz_port};
use super::*;

/// `routePath` for one edge: its route kind, ports and the box positions `p`.
pub fn route_with(
    kind: &Kind,
    po: &HashMap<(usize, End), Port>,
    dpos: &[Option<(f64, f64, f64)>],
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
    match kind {
        Kind::Bez => Some(Route {
            segs: vec![bz_ctl(
                port(a, End::A),
                side(End::A),
                port(b, End::B),
                side(End::B),
            )],
        }),
        Kind::Arc { top, level } => {
            let (pa, pb) = (port(a, End::A), port(b, End::B));
            let sg = if *top { -1.0 } else { 1.0 };
            let h = (14.0 + 13.0 * *level as f64) / 0.75;
            Some(Route {
                segs: vec![[
                    pa,
                    Pt {
                        x: pa.x,
                        y: pa.y + sg * h,
                    },
                    Pt {
                        x: pb.x,
                        y: pb.y + sg * h,
                    },
                    pb,
                ]],
            })
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
            let lat = (*idx as f64 - (*cnt as f64 - 1.0) / 2.0) * LANE;
            // (x, y, half height; None for the end ports)
            let mut c0: Vec<(f64, f64, Option<f64>)> = vec![(u_pt.x, u_pt.y, None)];
            for &k in &chain[1..chain.len() - 1] {
                let (dx, dy, dh) = dpos.get(k).copied().flatten()?;
                c0.push((dx + lat, dy + dh / 2.0, Some(dh / 2.0)));
            }
            c0.push((l_pt.x, l_pt.y, None));
            if *up_end == End::B {
                c0.reverse();
            }
            // One smooth curve: monotone cubic x(y); vertical tangents at ports; through a dummy
            // the edge is a slightly tilted straight segment across the row band.
            let n0 = c0.len();
            let sc: Vec<f64> = (0..n0 - 1)
                .map(|i| {
                    let dy = c0[i + 1].1 - c0[i].1;
                    if dy.abs() < 1.0 {
                        0.0
                    } else {
                        (c0[i + 1].0 - c0[i].0) / dy
                    }
                })
                .collect();
            let mut arr: Vec<(f64, f64, f64)> = Vec::new();
            for (i, &(x, y, hh)) in c0.iter().enumerate() {
                let Some(hh) = hh else {
                    arr.push((x, y, 0.0));
                    continue;
                };
                let (sa, sb) = (sc[i - 1], sc[i]);
                let m = if sa * sb <= 0.0 {
                    0.0
                } else {
                    (2.0 * sa * sb / (sa + sb)).clamp(-0.2, 0.2)
                };
                let dir = if c0[i + 1].1 >= c0[i - 1].1 {
                    1.0
                } else {
                    -1.0
                };
                let hh = hh * dir;
                arr.push((x - m * hh, y - hh, m));
                arr.push((x + m * hh, y + hh, m));
            }
            let segs = arr
                .windows(2)
                .map(|w| {
                    let (p, q) = (w[0], w[1]);
                    let h = q.1 - p.1;
                    [
                        Pt { x: p.0, y: p.1 },
                        Pt {
                            x: p.0 + p.2 * h / 3.0,
                            y: p.1 + h / 3.0,
                        },
                        Pt {
                            x: q.0 - q.2 * h / 3.0,
                            y: q.1 - h / 3.0,
                        },
                        Pt { x: q.0, y: q.1 },
                    ]
                })
                .collect();
            Some(Route { segs })
        }
    }
}

/// `routePath(e, L, P)`.
pub fn route_path(l: &Layout, e: usize, p: &[Rect]) -> Option<Route> {
    route_with(l.kinds[e].as_ref()?, &l.po, &l.dpos, e, l.edges[e], p)
}
