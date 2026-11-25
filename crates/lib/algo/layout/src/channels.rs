//! Channel track assignment (left-edge interval packing) for orthogonal routes.
//!
//! Every horizontal bend in an inter-row channel `[y_lo, y_hi]` gets an x-interval; segments
//! whose x-intervals overlap are placed on distinct vertical fractions inside that channel so
//! parallel horizontal rails never lie on top of one another.

use super::align::align_layout;
use super::geometry::bz_port;
use super::path::route_path;
use super::*;

/// Horizontal clearance between two segments packed onto the same channel track.
const TRACK_PAD: f64 = 14.0;

struct Hop {
    edge: usize,
    hop: usize,
    /// Upper-row x and lower-row x (in top-down order, so `x_hi_row < x_lo_row` means going right).
    x_top: f64,
    x_bot: f64,
    x_lo: f64,
    x_hi: f64,
    y_lo: f64,
    y_hi: f64,
}

/// Whether any non-empty row sits strictly between `ra` and `rb`.
fn spans_row(rows: &[RowOut], ra: usize, rb: usize) -> bool {
    let (lo, hi) = (ra.min(rb), ra.max(rb));
    lo + 1 < hi && rows[lo + 1..hi].iter().any(|r| !r.ids.is_empty())
}

fn edge_port(l: &Layout, e: usize, n: &Rect, end: End) -> (Pt, Side) {
    let pt = l.po.get(&(e, end)).copied().unwrap_or(Port {
        side: Side::T,
        off: 0.0,
    });
    (bz_port(n, pt.side, pt.off), pt.side)
}

fn push_hop_if_bent(
    hops: &mut Vec<Hop>,
    edge: usize,
    hop: usize,
    p: Pt,
    q: Pt,
    y_lo: f64,
    y_hi: f64,
) {
    if (p.x - q.x).abs() < 1.0 {
        return;
    }
    let (x_top, x_bot) = if p.y <= q.y { (p.x, q.x) } else { (q.x, p.x) };
    hops.push(Hop {
        edge,
        hop,
        x_top,
        x_bot,
        x_lo: p.x.min(q.x),
        x_hi: p.x.max(q.x),
        y_lo,
        y_hi,
    });
}

fn collect_chain_hops(
    l: &Layout,
    e: usize,
    chain: &[usize],
    up_end: End,
    lat: f64,
    hops: &mut Vec<Hop>,
) {
    let (Some(u), Some(lo)) = (l.pos.get(chain[0]), l.pos.get(chain[chain.len() - 1])) else {
        return;
    };
    let r_u = row_of_brick(&l.rows, chain[0]);
    let r_lo = row_of_brick(&l.rows, chain[chain.len() - 1]);
    let (u_pt, _) = edge_port(l, e, u, up_end);
    let (l_pt, _) = edge_port(l, e, lo, up_end.other());
    let mut c0: Vec<(f64, f64, Option<f64>)> = vec![(u_pt.x, u_pt.y, None)];
    for &k in &chain[1..chain.len() - 1] {
        let Some((dx, dy, dh)) = l.dpos.get(k).copied().flatten() else {
            return;
        };
        c0.push((dx + lat, dy + dh / 2.0, Some(dh / 2.0)));
    }
    c0.push((l_pt.x, l_pt.y, None));
    if up_end == End::B {
        c0.reverse();
    }
    let mut prev: Option<Pt> = None;
    for (i, &(x, y, hh)) in c0.iter().enumerate() {
        let (enter, leave) = match hh {
            None => (Pt { x, y }, Pt { x, y }),
            Some(hh) => {
                let dir = if c0[i + 1].1 >= c0[i - 1].1 {
                    1.0
                } else {
                    -1.0
                };
                (Pt { x, y: y - hh * dir }, Pt { x, y: y + hh * dir })
            }
        };
        if let Some(p) = prev {
            let band = match (up_end, r_u, r_lo) {
                (End::A, Some(ru), _) if ru + i < l.rows.len() => Some((
                    l.rows[ru + i - 1].y + l.rows[ru + i - 1].h,
                    l.rows[ru + i].y,
                )),
                (End::B, _, Some(rl)) if rl >= i && rl - i + 1 < l.rows.len() => {
                    Some((l.rows[rl - i].y + l.rows[rl - i].h, l.rows[rl - i + 1].y))
                }
                _ => None,
            };
            let (y_lo, y_hi) = band.unwrap_or_else(|| (p.y.min(enter.y), p.y.max(enter.y)));
            push_hop_if_bent(hops, e, i, p, enter, y_lo, y_hi);
        }
        prev = Some(if hh.is_some() { leave } else { enter });
    }
}

fn collect_channel_hops(l: &Layout) -> Vec<Hop> {
    let mut hops: Vec<Hop> = Vec::new();
    for e in 0..l.edges.len() {
        let Some(kind) = &l.kinds[e] else { continue };
        let (ia, ib) = l.edges[e];
        let (Some(a), Some(b)) = (l.pos.get(ia), l.pos.get(ib)) else {
            continue;
        };
        match kind {
            Kind::Bez => {
                let ((pa, sa), (pb, sb)) = (edge_port(l, e, a, End::A), edge_port(l, e, b, End::B));
                if matches!((sa, sb), (Side::B, Side::T) | (Side::T, Side::B))
                    && let (Some(ra), Some(rb)) =
                        (row_of_brick(&l.rows, ia), row_of_brick(&l.rows, ib))
                    && ra != rb
                    && !spans_row(&l.rows, ra, rb)
                {
                    let (lo, hi) = (ra.min(rb), ra.max(rb));
                    push_hop_if_bent(
                        &mut hops,
                        e,
                        0,
                        pa,
                        pb,
                        l.rows[lo].y + l.rows[lo].h,
                        l.rows[hi].y,
                    );
                }
            }
            Kind::Chain {
                chain,
                up_end,
                idx,
                cnt,
            } => {
                let lat = (*idx as f64 - (*cnt as f64 - 1.0) / 2.0) * LANE;
                collect_chain_hops(l, e, chain, *up_end, lat, &mut hops);
            }
            Kind::Arc { .. } => {}
        }
    }
    hops
}

/// Pack hops in one channel onto non-overlapping horizontal tracks in VCG priority order.
fn pack_channel_tracks(mut list: Vec<Hop>) -> Vec<Vec<Hop>> {
    list.sort_by(|a, b| {
        let key = |h: &Hop| {
            let right = h.x_top < h.x_bot;
            let prio = if right { -h.x_top } else { h.x_top };
            (right, prio)
        };
        let (ra, pa) = key(a);
        let (rb, pb) = key(b);
        ra.cmp(&rb)
            .then(pa.total_cmp(&pb))
            .then((a.edge, a.hop).cmp(&(b.edge, b.hop)))
    });
    let mut track_hops: Vec<Vec<Hop>> = Vec::new();
    for h in list {
        let slot = track_hops.iter().position(|used| {
            used.iter()
                .all(|u| u.x_hi + TRACK_PAD <= h.x_lo || h.x_hi + TRACK_PAD <= u.x_lo)
        });
        match slot {
            Some(t) => track_hops[t].push(h),
            None => track_hops.push(vec![h]),
        }
    }
    track_hops
}

fn track_pair_crossings(upper: &[Hop], lower: &[Hop]) -> usize {
    let mut c = 0;
    for a in upper {
        for b in lower {
            if a.x_lo + 1.0 < b.x_top && b.x_top < a.x_hi - 1.0 {
                c += 1;
            }
            if b.x_lo + 1.0 < a.x_bot && a.x_bot < b.x_hi - 1.0 {
                c += 1;
            }
        }
    }
    c
}

fn total_track_crossings(track_hops: &[Vec<Hop>], ord: &[usize]) -> usize {
    let mut c = 0;
    for i in 0..ord.len() {
        for j in i + 1..ord.len() {
            c += track_pair_crossings(&track_hops[ord[i]], &track_hops[ord[j]]);
        }
    }
    c
}

/// 2-opt track ordering to minimise exact vertical-vs-horizontal crossings inside one channel.
fn optimize_track_order(track_hops: &[Vec<Hop>]) -> Vec<usize> {
    let n = track_hops.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut best = total_track_crossings(track_hops, &order);
    for _pass in 0..8 {
        let mut improved = false;
        for i in 0..n {
            for j in i + 1..n {
                order.swap(i, j);
                let cur = total_track_crossings(track_hops, &order);
                if cur < best {
                    best = cur;
                    improved = true;
                } else {
                    order.swap(i, j);
                }
            }
        }
        if !improved {
            break;
        }
    }
    order
}

/// Assign non-overlapping horizontal channel tracks to every orthogonal edge and refresh routes.
pub(crate) fn assign_channels(l: &mut Layout) {
    align_layout(l);
    l.tracks.clear();
    let mut by_channel: HashMap<(i64, i64), Vec<Hop>> = HashMap::new();
    for h in collect_channel_hops(l) {
        let key = (h.y_lo.round() as i64, h.y_hi.round() as i64);
        by_channel.entry(key).or_default().push(h);
    }
    for list in by_channel.into_values() {
        let track_hops = pack_channel_tracks(list);
        let order = optimize_track_order(&track_hops);
        let n = track_hops.len();
        for (rank, &ti) in order.iter().enumerate() {
            let frac = if n <= 1 {
                0.5
            } else {
                0.18 + 0.64 * (rank as f64) / ((n - 1) as f64)
            };
            for h in &track_hops[ti] {
                l.tracks.insert((h.edge, h.hop), frac);
            }
        }
    }
    for e in 0..l.edges.len() {
        l.routes[e] = route_path(l, e, &l.pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn horiz_track_y(r: &Route) -> Option<f64> {
        r.segs.iter().find_map(|s| {
            ((s[0].y - s[3].y).abs() < 0.5 && (s[0].x - s[3].x).abs() > 1.0).then_some(s[0].y)
        })
    }

    #[test]
    fn upward_and_downward_edges_in_same_channel_get_distinct_tracks() {
        let input = Input {
            rows: vec!["top".into(), "bot".into()],
            bricks: vec![
                InBrick {
                    id: "t0".into(),
                    row: 0,
                    min_w: None,
                    min_h: None,
                },
                InBrick {
                    id: "t1".into(),
                    row: 0,
                    min_w: None,
                    min_h: None,
                },
                InBrick {
                    id: "b0".into(),
                    row: 1,
                    min_w: None,
                    min_h: None,
                },
                InBrick {
                    id: "b1".into(),
                    row: 1,
                    min_w: None,
                    min_h: None,
                },
            ],
            edges: vec![
                InEdge {
                    id: "t0|b0".into(),
                    a: 0,
                    b: 2,
                },
                InEdge {
                    id: "t1|b1".into(),
                    a: 1,
                    b: 3,
                },
                // Downward diagonal across the inter-row channel.
                InEdge {
                    id: "t0|b1".into(),
                    a: 0,
                    b: 3,
                },
                // Upward diagonal across the same inter-row channel.
                InEdge {
                    id: "b0|t1".into(),
                    a: 2,
                    b: 1,
                },
            ],
        };
        let l = layout(&input);
        let y_down = horiz_track_y(l.routes[2].as_ref().expect("downward route"))
            .expect("downward edge has horizontal channel track");
        let y_up = horiz_track_y(l.routes[3].as_ref().expect("upward route"))
            .expect("upward edge has horizontal channel track");
        assert!(
            (y_down - y_up).abs() > 5.0,
            "downward track ({y_down}) and upward track ({y_up}) collided in the same channel"
        );
    }
}
