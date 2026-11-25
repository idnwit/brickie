//! Brick widening, single-row column alignment, and port/dummy straightening before channel
//! track assignment.

use super::geometry::bz_port;
use super::*;

/// Minimum clearance from a dummy corridor to a brick box or neighbouring dummy.
const DUMMY_BOX_PAD: f64 = 18.0;
const DUMMY_PEER_PAD: f64 = 12.0;
/// Cap on how wide a brick may stretch to align with its adjacent-row neighbours.
const MAX_WIDEN: f64 = 380.0;

/// Direct adjacent-row neighbour centres per brick, plus whether each brick has any edge.
fn collect_direct_targets(l: &Layout) -> (Vec<Vec<f64>>, Vec<bool>) {
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
    let mut connected = vec![false; l.pos.len()];
    for &(a, b) in &l.edges {
        if a != b {
            connected[a] = true;
            connected[b] = true;
        }
    }
    (targets, connected)
}

/// Shift vertical ports of brick `b` after its horizontal centre moved by `-shift`.
fn shift_brick_ports(l: &mut Layout, b: usize, shift: f64, clamp_half: Option<f64>) {
    for (&(oe, oend), op) in l.po.iter_mut() {
        if op.side.horizontal() {
            continue;
        }
        let ob = if oend == End::A {
            l.edges[oe].0
        } else {
            l.edges[oe].1
        };
        if ob != b {
            continue;
        }
        let next = op.off + shift;
        op.off = match clamp_half {
            Some(half) => next.clamp(-half, half),
            None => next,
        };
    }
}

/// Horizontal free span `[room_lo, room_hi]` and dummy ceiling `dummy_hi` around brick `b` in `row`.
fn row_free_room(l: &Layout, row: &RowOut, b: usize, connected: &[bool]) -> (f64, f64, f64, bool) {
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
        if dy + dh <= row.y + 1.0 || dy >= row.y + row.h - 1.0 {
            continue;
        }
        if dx <= r.x + 0.5 {
            room_lo = room_lo.max(dx + DUMMY_BOX_PAD);
        } else if dx >= r.x + r.w - 0.5 {
            dummy_hi = dummy_hi.min(dx - DUMMY_BOX_PAD);
        }
    }
    (room_lo, room_hi, dummy_hi, right_unconnected)
}

/// Stretch one brick `row.ids[pos_in_row]` within its free row space to overlap `targets`.
fn widen_row_brick(
    l: &mut Layout,
    row: &RowOut,
    pos_in_row: usize,
    targets: &[f64],
    connected: &[bool],
) {
    let b = row.ids[pos_in_row];
    let r = l.pos[b];
    let (room_lo, mut room_hi, dummy_hi, right_unconnected) = row_free_room(l, row, b, connected);
    let want_lo = targets
        .iter()
        .copied()
        .fold(r.x, |acc, tx| acc.min(tx - PAD - 8.0));
    let want_hi = targets
        .iter()
        .copied()
        .fold(r.x + r.w, |acc, tx| acc.max(tx + PAD + 8.0));
    if right_unconnected
        && want_hi > room_hi
        && row.ids[pos_in_row + 1..].iter().all(|&s| !connected[s])
    {
        let target_hi = want_hi.min((r.x + MAX_WIDEN.max(r.w)).min(dummy_hi));
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
    room_hi = room_hi.min(dummy_hi);
    let new_lo = want_lo.max(room_lo).min(r.x);
    let new_hi = want_hi.min(room_hi).max(r.x + r.w);
    let mut new_w = (new_hi - new_lo).min(MAX_WIDEN.max(r.w));
    let mut new_x = new_lo;
    if new_x + new_w < r.x + r.w {
        new_x = (r.x + r.w - new_w).max(room_lo);
    }
    if new_x > r.x {
        new_x = r.x;
    }
    new_w = new_w.min(room_hi - new_x).max(r.w);
    if (new_w - r.w).abs() >= 1.0 {
        let shift = (r.x + r.w / 2.0) - (new_x + new_w / 2.0);
        l.pos[b].x = new_x;
        l.pos[b].w = new_w;
        shift_brick_ports(l, b, shift, None);
    }
}

/// Stretch bricks horizontally within their free row space (never colliding with sibling bricks or
/// dummy corridors) so they overlap their direct adjacent-row neighbours; this lets `align_ports`
/// turn bent edges into straight vertical lines and keeps sparse rows visually balanced.
fn widen_bricks(l: &mut Layout) {
    let (targets, connected) = collect_direct_targets(l);
    let rows = l.rows.clone();
    for row in &rows {
        for (pos_in_row, &b) in row.ids.iter().enumerate() {
            if !targets[b].is_empty() {
                widen_row_brick(l, row, pos_in_row, &targets[b], &connected);
            }
        }
    }
    align_single_rows(l);
}

/// Push left/right bypass dummies in row `ri` outside `[col_x, col_x + col_w]`.
fn push_row_dummies(l: &mut Layout, ri: usize, old_x: f64, col_x: f64, col_w: f64) {
    let (ry, rh) = (l.rows[ri].y, l.rows[ri].h);
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

/// Align one run of >= 2 consecutive single-brick rows onto a shared vertical column.
fn align_single_run(l: &mut Layout, run: &[(usize, usize)]) {
    if run.len() < 2 {
        return;
    }
    let col_w = run.iter().map(|&(_, b)| l.pos[b].w).fold(0.0_f64, f64::max);
    let avg_cx = run
        .iter()
        .map(|&(_, b)| l.pos[b].x + l.pos[b].w / 2.0)
        .sum::<f64>()
        / run.len() as f64;
    let max_left_dummies = run
        .iter()
        .map(|&(ri, b)| {
            let (ry, rh, bx) = (l.rows[ri].y, l.rows[ri].h, l.pos[b].x);
            l.dpos
                .iter()
                .flatten()
                .filter(|&&(dx, dy, dh)| dy + dh > ry + 1.0 && dy < ry + rh - 1.0 && dx <= bx + 0.5)
                .count()
        })
        .max()
        .unwrap_or(0);
    let left_pad = if max_left_dummies > 0 {
        DUMMY_BOX_PAD + (max_left_dummies - 1) as f64 * DUMMY_PEER_PAD
    } else {
        0.0
    };
    let col_x = (avg_cx - col_w / 2.0).max(l.px0 + left_pad);
    for &(ri, b) in run {
        let old_x = l.pos[b].x;
        let shift = (old_x + l.pos[b].w / 2.0) - (col_x + col_w / 2.0);
        l.pos[b].x = col_x;
        l.pos[b].w = col_w;
        shift_brick_ports(l, b, shift, Some(col_w / 2.0 - PAD));
        push_row_dummies(l, ri, old_x, col_x, col_w);
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

/// Align consecutive 1-brick rows onto a shared vertical column, nudging left/right bypass
/// dummies just outside the column so linear stacks form one crisp vertical axis. Wide nodes
/// (`w > MAX_WIDEN`, e.g. group containers) break runs instead of stretching adjacent rows.
fn align_single_rows(l: &mut Layout) {
    let mut run: Vec<(usize, usize)> = Vec::new();
    for ri in 0..l.rows.len() {
        if l.rows[ri].ids.len() != 1 || l.pos[l.rows[ri].ids[0]].w > MAX_WIDEN {
            align_single_run(l, &run);
            run.clear();
            continue;
        }
        run.push((ri, l.rows[ri].ids[0]));
    }
    align_single_run(l, &run);
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

/// Widen bricks, align single-brick rows, snap direct ports, and straighten dummy chains.
pub(crate) fn align_layout(l: &mut Layout) {
    widen_bricks(l);
    align_ports(l);
    align_dummies(l);
}
