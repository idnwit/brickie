//! Channel track assignment (left-edge interval packing) for orthogonal routes.
//!
//! Every horizontal bend in an inter-row channel `[y_lo, y_hi]` gets an x-interval; segments
//! whose x-intervals overlap are placed on distinct vertical fractions inside that channel so
//! parallel horizontal rails never lie on top of one another.

use super::geometry::bz_port;
use super::path::route_path;
use super::*;

/// Horizontal clearance between two segments packed onto the same channel track.
const TRACK_PAD: f64 = 14.0;
/// Minimum clearance from a dummy corridor to a brick box or neighbouring dummy.
const DUMMY_BOX_PAD: f64 = 18.0;
const DUMMY_PEER_PAD: f64 = 12.0;
/// Cap on how wide a brick may stretch to align with its adjacent-row neighbours.
const MAX_WIDEN: f64 = 380.0;

/// Stretch bricks horizontally within their free row space (never colliding with sibling bricks or
/// dummy corridors) so they overlap their direct adjacent-row neighbours; this lets `align_ports`
/// turn bent edges into straight vertical lines and keeps sparse rows visually balanced.
fn widen_bricks(l: &mut Layout) {
    // Collect direct adjacent-row neighbour centres per brick.
    let mut targets: Vec<Vec<f64>> = vec![Vec::new(); l.pos.len()];
    for e in 0..l.edges.len() {
        let is_direct = match &l.kinds[e] {
            Some(Kind::Bez) => true,
            Some(Kind::Chain { chain, .. }) => chain.len() == 2,
            _ => false,
        };
        if !is_direct {
            continue;
        }
        let (a, b) = l.edges[e];
        let (ra, rb) = (l.pos[a], l.pos[b]);
        if (ra.y - rb.y).abs() < 1.0 {
            continue;
        }
        targets[a].push(rb.x + rb.w / 2.0);
        targets[b].push(ra.x + ra.w / 2.0);
    }

    // Also track which bricks participate in any edge at all (unconnected bricks may slide right
    // when a connected sibling wants to widen under its adjacent-row neighbours).
    let mut connected = vec![false; l.pos.len()];
    for &(a, b) in &l.edges {
        if a != b {
            connected[a] = true;
            connected[b] = true;
        }
    }

    for row in &l.rows {
        let ry = row.y;
        let rh = row.h;
        for (pos_in_row, &b) in row.ids.iter().enumerate() {
            if targets[b].is_empty() {
                continue;
            }
            let r = l.pos[b];
            let mut room_lo = l.px0;
            let mut room_hi = l.w - PADR;
            let mut right_unconnected = false;
            for &sib in &row.ids {
                if sib == b {
                    continue;
                }
                let sr = l.pos[sib];
                if sr.x + sr.w <= r.x + 0.5 {
                    room_lo = room_lo.max(sr.x + sr.w + GAP);
                } else if sr.x >= r.x + r.w - 0.5 {
                    let limit = sr.x - GAP;
                    if limit < room_hi {
                        room_hi = limit;
                        right_unconnected = !connected[sib];
                    }
                }
            }
            let mut dummy_hi = f64::INFINITY;
            for &(dx, dy, dh) in l.dpos.iter().flatten() {
                if dy + dh <= ry + 1.0 || dy >= ry + rh - 1.0 {
                    continue;
                }
                if dx <= r.x + 0.5 {
                    room_lo = room_lo.max(dx + DUMMY_BOX_PAD);
                } else if dx >= r.x + r.w - 0.5 {
                    dummy_hi = dummy_hi.min(dx - DUMMY_BOX_PAD);
                }
            }
            let want_lo = targets[b]
                .iter()
                .copied()
                .fold(r.x, |acc, tx| acc.min(tx - PAD - 8.0));
            let want_hi = targets[b]
                .iter()
                .copied()
                .fold(r.x + r.w, |acc, tx| acc.max(tx + PAD + 8.0));
            // If only unconnected siblings sit to the right (and no dummy blocks), nudge them right
            // so `b` can reach its adjacent-row neighbour.
            if right_unconnected && want_hi > room_hi {
                let all_tail_free = row.ids[pos_in_row + 1..].iter().all(|&s| !connected[s]);
                if all_tail_free {
                    let cap_hi = (r.x + MAX_WIDEN.max(r.w)).min(dummy_hi);
                    let target_hi = want_hi.min(cap_hi);
                    if target_hi > room_hi {
                        let push = target_hi - room_hi;
                        for &s in &row.ids[pos_in_row + 1..] {
                            l.pos[s].x += push;
                        }
                        room_hi = target_hi;
                        let max_x = l.pos.iter().map(|p| p.x + p.w).fold(0.0_f64, f64::max);
                        l.w = l.w.max(max_x + PADR);
                    }
                }
            }
            room_hi = room_hi.min(dummy_hi);
            let new_lo = want_lo.max(room_lo).min(r.x);
            let new_hi = want_hi.min(room_hi).max(r.x + r.w);
            let mut new_w = (new_hi - new_lo).min(MAX_WIDEN.max(r.w));
            // Keep existing box covered if MAX_WIDEN clamped the growth.
            let mut new_x = new_lo;
            if new_x + new_w < r.x + r.w {
                new_x = (r.x + r.w - new_w).max(room_lo);
            }
            if new_x > r.x {
                new_x = r.x;
            }
            new_w = new_w.min(room_hi - new_x).max(r.w);
            let old_cx = r.x + r.w / 2.0;
            let new_cx = new_x + new_w / 2.0;
            if (new_w - r.w).abs() >= 1.0 {
                let shift = old_cx - new_cx;
                l.pos[b].x = new_x;
                l.pos[b].w = new_w;
                for (&(oe, oend), op) in l.po.iter_mut() {
                    if op.side.horizontal() {
                        continue;
                    }
                    let ob = if oend == End::A {
                        l.edges[oe].0
                    } else {
                        l.edges[oe].1
                    };
                    if ob == b {
                        op.off += shift;
                    }
                }
            }
        }
    }
    // Align consecutive single-brick rows onto one straight central column (eliminates zig-zagging
    // in linear stacks like `app -> inbound -> outbound -> web_ui -> domain -> shared`).
    align_single_rows(l);
}

/// Align consecutive 1-brick rows onto a shared vertical column, nudging left/right bypass
/// dummies just outside the column so linear stacks (`console`) form one crisp vertical axis
/// instead of zig-zagging.
fn align_single_rows(l: &mut Layout) {
    let mut run: Vec<(usize, usize)> = Vec::new();
    let flush = |l: &mut Layout, run: &mut Vec<(usize, usize)>| {
        if run.len() >= 2 {
            let max_w = run.iter().map(|&(_, b)| l.pos[b].w).fold(0.0_f64, f64::max);
            let avg_cx: f64 = run
                .iter()
                .map(|&(_, b)| l.pos[b].x + l.pos[b].w / 2.0)
                .sum::<f64>()
                / run.len() as f64;
            let mut max_left_dummies = 0usize;
            for &(ri, b) in run.iter() {
                let (ry, rh) = (l.rows[ri].y, l.rows[ri].h);
                let bx = l.pos[b].x;
                let left_cnt = l
                    .dpos
                    .iter()
                    .flatten()
                    .filter(|&&(dx, dy, dh)| {
                        dy + dh > ry + 1.0 && dy < ry + rh - 1.0 && dx <= bx + 0.5
                    })
                    .count();
                max_left_dummies = max_left_dummies.max(left_cnt);
            }
            let min_col_x = l.px0
                + if max_left_dummies > 0 {
                    DUMMY_BOX_PAD + (max_left_dummies - 1) as f64 * DUMMY_PEER_PAD
                } else {
                    0.0
                };
            let col_x = (avg_cx - max_w / 2.0).max(min_col_x);
            let col_w = max_w;
            for &(ri, b) in run.iter() {
                let (ry, rh) = (l.rows[ri].y, l.rows[ri].h);
                let old_x = l.pos[b].x;
                let old_cx = old_x + l.pos[b].w / 2.0;
                let new_cx = col_x + col_w / 2.0;
                let shift = old_cx - new_cx;
                l.pos[b].x = col_x;
                l.pos[b].w = col_w;
                for (&(oe, oend), op) in l.po.iter_mut() {
                    if op.side.horizontal() {
                        continue;
                    }
                    let ob = if oend == End::A {
                        l.edges[oe].0
                    } else {
                        l.edges[oe].1
                    };
                    if ob == b {
                        op.off = (op.off + shift).clamp(-(col_w / 2.0 - PAD), col_w / 2.0 - PAD);
                    }
                }
                let mut left_d: Vec<usize> = Vec::new();
                let mut right_d: Vec<usize> = Vec::new();
                for (di, slot) in l.dpos.iter().enumerate() {
                    let Some((dx, dy, dh)) = *slot else { continue };
                    if dy + dh <= ry + 1.0 || dy >= ry + rh - 1.0 {
                        continue;
                    }
                    if dx <= old_x + 0.5 {
                        left_d.push(di);
                    } else {
                        right_d.push(di);
                    }
                }
                left_d.sort_by(|&a, &b| l.dpos[b].unwrap().0.total_cmp(&l.dpos[a].unwrap().0));
                for (idx, di) in left_d.into_iter().enumerate() {
                    let (dx, dy, dh) = l.dpos[di].unwrap();
                    let limit = col_x - DUMMY_BOX_PAD - idx as f64 * DUMMY_PEER_PAD;
                    if dx > limit {
                        l.dpos[di] = Some((limit, dy, dh));
                    }
                }
                right_d.sort_by(|&a, &b| l.dpos[a].unwrap().0.total_cmp(&l.dpos[b].unwrap().0));
                for (idx, di) in right_d.into_iter().enumerate() {
                    let (dx, dy, dh) = l.dpos[di].unwrap();
                    let limit = col_x + col_w + DUMMY_BOX_PAD + idx as f64 * DUMMY_PEER_PAD;
                    if dx < limit {
                        l.dpos[di] = Some((limit, dy, dh));
                    }
                }
            }
            let max_x = l
                .pos
                .iter()
                .map(|r| r.x + r.w)
                .chain(
                    l.dpos
                        .iter()
                        .flatten()
                        .map(|&(dx, _, _)| dx + DUMMY_BOX_PAD),
                )
                .fold(0.0_f64, f64::max);
            l.w = l.w.max(max_x + PADR);
        }
        run.clear();
    };
    for ri in 0..l.rows.len() {
        if l.rows[ri].ids.len() != 1 {
            flush(l, &mut run);
            continue;
        }
        run.push((ri, l.rows[ri].ids[0]));
    }
    flush(l, &mut run);
}

/// Free horizontal interval `[min_x, max_x]` that port `(e, end)` on `(brick, sd)` can slide
/// within without crossing its neighbouring ports or leaving `[brick.x + PAD, brick.x + brick.w - PAD]`.
fn port_bounds(l: &Layout, brick: usize, sd: Side, e: usize, end: End) -> (f64, f64) {
    let r = l.pos[brick];
    let mut lo = r.x + PAD;
    let mut hi = r.x + r.w - PAD;
    let cur_x = r.x + r.w / 2.0 + l.po[&(e, end)].off;
    for (&(oe, oend), &op) in &l.po {
        if (oe, oend) == (e, end) || op.side != sd {
            continue;
        }
        let ob = if oend == End::A {
            l.edges[oe].0
        } else {
            l.edges[oe].1
        };
        if ob != brick {
            continue;
        }
        let ox = r.x + r.w / 2.0 + op.off;
        if ox < cur_x {
            lo = lo.max(ox + PORT);
        } else if ox > cur_x {
            hi = hi.min(ox - PORT);
        }
    }
    (lo, hi)
}

/// Snap vertical-to-vertical ports of direct adjacent-row edges to the same `x` whenever their
/// free port ranges overlap, turning tiny Z-jogs into straight vertical lines (0 bends).
fn align_ports(l: &mut Layout) {
    for _pass in 0..2 {
        for e in 0..l.edges.len() {
            let is_direct = match &l.kinds[e] {
                Some(Kind::Bez) => true,
                Some(Kind::Chain { chain, .. }) => chain.len() == 2,
                _ => false,
            };
            if !is_direct {
                continue;
            }
            let (ia, ib) = l.edges[e];
            let (Some(&pa), Some(&pb)) = (l.po.get(&(e, End::A)), l.po.get(&(e, End::B))) else {
                continue;
            };
            if !matches!((pa.side, pb.side), (Side::B, Side::T) | (Side::T, Side::B)) {
                continue;
            }
            let (ra, rb) = (l.pos[ia], l.pos[ib]);
            let xa = ra.x + ra.w / 2.0 + pa.off;
            let xb = rb.x + rb.w / 2.0 + pb.off;
            if (xa - xb).abs() < 1.0 {
                continue;
            }
            let (alo, ahi) = port_bounds(l, ia, pa.side, e, End::A);
            let (blo, bhi) = port_bounds(l, ib, pb.side, e, End::B);
            let lo = alo.max(blo);
            let hi = ahi.min(bhi);
            if lo <= hi {
                // Pick the aligned x closest to the current midpoint (minimises displacement).
                let target = ((xa + xb) / 2.0).clamp(lo, hi);
                l.po.get_mut(&(e, End::A)).unwrap().off = target - (ra.x + ra.w / 2.0);
                l.po.get_mut(&(e, End::B)).unwrap().off = target - (rb.x + rb.w / 2.0);
            }
        }
    }
}

/// Free horizontal interval `[min_x, max_x]` for dummy node `k` in its row, clear of brick boxes
/// and neighbouring dummies.
fn dummy_bounds(l: &Layout, k: usize, dummy_ids: &[usize]) -> Option<(f64, f64)> {
    let (dx, dy, dh) = l.dpos[k]?;
    let mut min_x = l.px0;
    let mut max_x = l.w - PADR;
    for r in &l.pos {
        if r.y + r.h <= dy + 1.0 || r.y >= dy + dh - 1.0 {
            continue;
        }
        if r.x + r.w <= dx {
            min_x = min_x.max(r.x + r.w + DUMMY_BOX_PAD);
        } else if r.x >= dx {
            max_x = max_x.min(r.x - DUMMY_BOX_PAD);
        }
    }
    for &od in dummy_ids {
        if od == k {
            continue;
        }
        let Some((ox, oy, _)) = l.dpos[od] else {
            continue;
        };
        if (oy - dy).abs() > 2.0 {
            continue;
        }
        if ox < dx {
            min_x = min_x.max(ox + DUMMY_PEER_PAD);
        } else if ox > dx {
            max_x = max_x.min(ox - DUMMY_PEER_PAD);
        }
    }
    (min_x <= max_x).then_some((min_x, max_x))
}

/// Straighten multi-row dummy chains onto a single vertical `x` across all intermediate rows
/// whenever a common free corridor exists (snapping to the upper or lower port `x` when possible),
/// eliminating mid-chain zig-zags and up to 2 bends per chain.
fn align_dummies(l: &mut Layout) {
    let dummy_ids: Vec<usize> = (0..l.dpos.len()).filter(|&i| l.dpos[i].is_some()).collect();
    if dummy_ids.is_empty() {
        return;
    }
    for e in 0..l.edges.len() {
        let Some(Kind::Chain {
            chain,
            up_end,
            idx: _,
            cnt,
        }) = l.kinds[e].clone()
        else {
            continue;
        };
        if chain.len() <= 2 || cnt > 1 {
            continue;
        }
        let (Some(u), Some(lo)) = (l.pos.get(chain[0]), l.pos.get(chain[chain.len() - 1])) else {
            continue;
        };
        let u_pt = bz_port(
            u,
            l.po.get(&(e, up_end)).map_or(Side::B, |p| p.side),
            l.po.get(&(e, up_end)).map_or(0.0, |p| p.off),
        );
        let l_pt = bz_port(
            lo,
            l.po.get(&(e, up_end.other())).map_or(Side::T, |p| p.side),
            l.po.get(&(e, up_end.other())).map_or(0.0, |p| p.off),
        );
        let mids = &chain[1..chain.len() - 1];
        let bounds: Vec<Option<(f64, f64)>> = mids
            .iter()
            .map(|&k| dummy_bounds(l, k, &dummy_ids))
            .collect();
        // Try one common straight x across the whole dummy chain first.
        let mut common_lo = f64::NEG_INFINITY;
        let mut common_hi = f64::INFINITY;
        for b in &bounds {
            if let Some((a, z)) = *b {
                common_lo = common_lo.max(a);
                common_hi = common_hi.min(z);
            } else {
                common_lo = 1.0;
                common_hi = 0.0;
                break;
            }
        }
        if common_lo <= common_hi {
            let u_brick = chain[0];
            let l_brick = chain[chain.len() - 1];
            let u_sd = l.po.get(&(e, up_end)).map_or(Side::B, |p| p.side);
            let l_sd = l.po.get(&(e, up_end.other())).map_or(Side::T, |p| p.side);
            let (u_lo, u_hi) = if !u_sd.horizontal() && l.po.contains_key(&(e, up_end)) {
                port_bounds(l, u_brick, u_sd, e, up_end)
            } else {
                (u_pt.x, u_pt.x)
            };
            let (l_lo, l_hi) = if !l_sd.horizontal() && l.po.contains_key(&(e, up_end.other())) {
                port_bounds(l, l_brick, l_sd, e, up_end.other())
            } else {
                (l_pt.x, l_pt.x)
            };
            // Best case: one vertical x that aligns upper port, all dummies, AND lower port (0 bends!).
            let tri_lo = common_lo.max(u_lo).max(l_lo);
            let tri_hi = common_hi.min(u_hi).min(l_hi);
            let nx = if tri_lo <= tri_hi {
                ((u_pt.x + l_pt.x) / 2.0).clamp(tri_lo, tri_hi)
            } else if u_pt.x >= common_lo && u_pt.x <= common_hi {
                u_pt.x
            } else if l_pt.x >= common_lo && l_pt.x <= common_hi {
                l_pt.x
            } else {
                ((u_pt.x + l_pt.x) / 2.0).clamp(common_lo, common_hi)
            };
            for &k in mids {
                if let Some((_, dy, dh)) = l.dpos[k] {
                    l.dpos[k] = Some((nx, dy, dh));
                }
            }
            if !u_sd.horizontal()
                && nx >= u_lo
                && nx <= u_hi
                && let Some(p) = l.po.get_mut(&(e, up_end))
            {
                p.off = nx - (l.pos[u_brick].x + l.pos[u_brick].w / 2.0);
            }
            if !l_sd.horizontal()
                && nx >= l_lo
                && nx <= l_hi
                && let Some(p) = l.po.get_mut(&(e, up_end.other()))
            {
                p.off = nx - (l.pos[l_brick].x + l.pos[l_brick].w / 2.0);
            }
            continue;
        }
        // Fallback: align each dummy individually toward upper or lower port.
        for (&k, b) in mids.iter().zip(&bounds) {
            let (Some((dx, dy, dh)), Some((min_x, max_x))) = (l.dpos[k], *b) else {
                continue;
            };
            let cand = [u_pt.x, l_pt.x]
                .into_iter()
                .filter(|&tx| tx >= min_x && tx <= max_x)
                .min_by(|a, b| (a - dx).abs().total_cmp(&(b - dx).abs()));
            if let Some(nx) = cand {
                l.dpos[k] = Some((nx, dy, dh));
            }
        }
    }
}

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

/// Whether any brick box sits vertically inside `(y_lo, y_hi)`.
fn spans_row(pos: &[Rect], y_lo: f64, y_hi: f64) -> bool {
    pos.iter()
        .any(|r| r.y + r.h > y_lo + 4.0 && r.y < y_hi - 4.0)
}

/// Assign non-overlapping horizontal channel tracks to every orthogonal edge and refresh routes.
pub(crate) fn assign_channels(l: &mut Layout) {
    widen_bricks(l);
    align_ports(l);
    align_dummies(l);
    l.tracks.clear();
    let mut hops: Vec<Hop> = Vec::new();
    for e in 0..l.edges.len() {
        let Some(kind) = &l.kinds[e] else { continue };
        let (ia, ib) = l.edges[e];
        let (Some(a), Some(b)) = (l.pos.get(ia), l.pos.get(ib)) else {
            continue;
        };
        let port = |n: &Rect, end: End| {
            let pt = l.po.get(&(e, end)).copied().unwrap_or(Port {
                side: Side::T,
                off: 0.0,
            });
            (bz_port(n, pt.side, pt.off), pt.side)
        };
        match kind {
            Kind::Bez => {
                let ((pa, sa), (pb, sb)) = (port(a, End::A), port(b, End::B));
                if matches!((sa, sb), (Side::B, Side::T) | (Side::T, Side::B))
                    && (pa.x - pb.x).abs() >= 1.0
                {
                    let (y_lo, y_hi) = (pa.y.min(pb.y), pa.y.max(pb.y));
                    if !spans_row(&l.pos, y_lo, y_hi) {
                        let (x_top, x_bot) = if pa.y <= pb.y {
                            (pa.x, pb.x)
                        } else {
                            (pb.x, pa.x)
                        };
                        hops.push(Hop {
                            edge: e,
                            hop: 0,
                            x_top,
                            x_bot,
                            x_lo: pa.x.min(pb.x),
                            x_hi: pa.x.max(pb.x),
                            y_lo,
                            y_hi,
                        });
                    }
                }
            }
            Kind::Chain {
                chain,
                up_end,
                idx,
                cnt,
            } => {
                let (Some(u), Some(lo)) = (l.pos.get(chain[0]), l.pos.get(chain[chain.len() - 1]))
                else {
                    continue;
                };
                let (u_pt, _) = port(u, *up_end);
                let (l_pt, _) = port(lo, up_end.other());
                let lat = (*idx as f64 - (*cnt as f64 - 1.0) / 2.0) * LANE;
                let mut c0: Vec<(f64, f64, Option<f64>)> = vec![(u_pt.x, u_pt.y, None)];
                let mut ok = true;
                for &k in &chain[1..chain.len() - 1] {
                    let Some((dx, dy, dh)) = l.dpos.get(k).copied().flatten() else {
                        ok = false;
                        break;
                    };
                    c0.push((dx + lat, dy + dh / 2.0, Some(dh / 2.0)));
                }
                if !ok {
                    continue;
                }
                c0.push((l_pt.x, l_pt.y, None));
                if *up_end == End::B {
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
                    if let Some(p) = prev
                        && (p.x - enter.x).abs() >= 1.0
                    {
                        let (x_top, x_bot) = if p.y <= enter.y {
                            (p.x, enter.x)
                        } else {
                            (enter.x, p.x)
                        };
                        hops.push(Hop {
                            edge: e,
                            hop: i,
                            x_top,
                            x_bot,
                            x_lo: p.x.min(enter.x),
                            x_hi: p.x.max(enter.x),
                            y_lo: p.y.min(enter.y),
                            y_hi: p.y.max(enter.y),
                        });
                    }
                    prev = Some(if hh.is_some() { leave } else { enter });
                }
            }
            Kind::Arc { .. } => {}
        }
    }

    // Group hops by channel `(y_lo, y_hi)` rounded to whole pixels.
    let mut by_channel: HashMap<(i64, i64), Vec<Hop>> = HashMap::new();
    for h in hops {
        let key = (h.y_lo.round() as i64, h.y_hi.round() as i64);
        by_channel.entry(key).or_default().push(h);
    }
    for mut list in by_channel.into_values() {
        // Vertical-constraint order to avoid artificial channel crossings:
        // - right-going hops (`x_top < x_bot`): larger `x_top` gets a higher track (smaller t)
        // - left-going hops (`x_top > x_bot`): smaller `x_top` gets a higher track (smaller t)
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
        // Pack hops onto non-overlapping horizontal tracks, then optimise track order (2-opt)
        // to minimise exact vertical-vs-horizontal crossings inside this channel.
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
        let pair_cross = |upper: &[Hop], lower: &[Hop]| -> usize {
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
        };
        let n = track_hops.len();
        let mut order: Vec<usize> = (0..n).collect();
        let total_cross = |ord: &[usize]| -> usize {
            let mut c = 0;
            for i in 0..ord.len() {
                for j in i + 1..ord.len() {
                    c += pair_cross(&track_hops[ord[i]], &track_hops[ord[j]]);
                }
            }
            c
        };
        let mut best = total_cross(&order);
        for _pass in 0..8 {
            let mut improved = false;
            for i in 0..n {
                for j in i + 1..n {
                    order.swap(i, j);
                    let cur = total_cross(&order);
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
