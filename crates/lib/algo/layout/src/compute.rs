//! `computeLayout` of the original: rows pinned by layer ([`rank`]), crossing minimisation
//! ([`order`]), x placement ([`place`]), same-row arcs, row y positions and initial ports.

use super::order::minimise;
use super::place::place_x;
use super::ports::initial_ports;
use super::rank::build;
use super::smart::smart_route;
use super::*;

/// `computeLayout`.
pub fn compute_layout(inp: &Input) -> Layout {
    let nb = inp.bricks.len();
    let super::rank::Built {
        mut ns,
        mut layers,
        used,
        es,
        mut arcs,
        mut kinds,
        wt,
    } = build(inp);

    // Crossing reduction.
    let cross0 = super::order::total_cross(&ns, &layers);
    let bc = minimise(&mut ns, &mut layers, nb);

    // x: weighted isotonic regression per layer.
    let (min_x, max_x) = place_x(&mut ns, &layers, &wt);

    // Same-row arcs: lowest free level above or below the row.
    let mut lvl: HashMap<(usize, bool), Vec<(usize, usize, usize)>> = HashMap::new();
    arcs.sort_by(|&p, &q| {
        let d = |e: usize| ns[inp.edges[e].a].i.abs_diff(ns[inp.edges[e].b].i);
        d(p).cmp(&d(q)).then(inp.edges[p].id.cmp(&inp.edges[q].id))
    });
    for &e in &arcs {
        let (a, b) = (inp.edges[e].a, inp.edges[e].b);
        let (lo, hi) = (ns[a].i.min(ns[b].i), ns[a].i.max(ns[b].i));
        let li = ns[a].li;
        let lv = |top: bool| {
            let m = lvl.get(&(li, top)).map_or(0, |v| {
                v.iter()
                    .filter(|o| o.0 <= hi && lo <= o.1)
                    .map(|o| o.2)
                    .max()
                    .unwrap_or(0)
            });
            m + 1
        };
        let (lt, lb) = (lv(true), lv(false));
        let top = lb >= lt;
        let l = if top { lt } else { lb };
        lvl.entry((li, top)).or_default().push((lo, hi, l));
        kinds[e] = Some(Kind::Arc { top, level: l });
    }
    let max_l = |li: usize, top: bool| {
        lvl.get(&(li, top))
            .map_or(0, |v| v.iter().map(|o| o.2).max().unwrap_or(0)) as f64
    };

    // Rows (y). The original starts at 72 for the zone panel labels; no zones here.
    // A gap no edge crosses needs room for the interface labels under the boxes only.
    let spans: Vec<(usize, usize)> = (inp.edges.iter())
        .map(|e| (ns[e.a].li, ns[e.b].li))
        .map(|(p, q)| (p.min(q), p.max(q)))
        .collect();
    let busy = |l: usize| spans.iter().any(|&(p, q)| p < l && l <= q);
    let mut rows: Vec<RowOut> = Vec::new();
    let mut y = 40.0;
    for (l, row) in layers.iter().enumerate() {
        if l > 0 {
            let gap = if busy(l) { GAPY } else { GAPY_FREE };
            y += H + gap + 12.0 * (max_l(l - 1, false) + max_l(l, true));
        } else {
            y += 12.0 * max_l(0, true);
        }
        for &n in row {
            ns[n].y = y;
            ns[n].h = H;
        }
        rows.push(RowOut {
            y,
            h: H,
            label: inp.rows.get(used[l]).cloned().unwrap_or_default(),
            ids: row.iter().filter(|&&n| ns[n].real).copied().collect(),
        });
    }
    let main_bottom = if rows.is_empty() { 60.0 } else { y + H };
    let x0 = PADL;
    let shift = x0 - min_x;
    for n in ns.iter_mut() {
        n.x += shift;
    }
    let main_w = max_x - min_x;

    let po = initial_ports(inp, &ns, &kinds, &es);

    // Output.
    let mut pos = Vec::new();
    let mut dpos = vec![None; ns.len()];
    for (i, n) in ns.iter().enumerate() {
        if n.real {
            pos.push(Rect {
                x: n.x - n.w / 2.0,
                y: n.y,
                w: n.w,
                h: n.h,
            });
        } else {
            dpos[i] = Some((n.x, n.y, n.h));
        }
    }
    let mut lay = Layout {
        w: x0 + main_w + PADR,
        h: main_bottom + 50.0,
        px0: x0,
        pos,
        dpos,
        rows,
        kinds,
        po,
        routes: vec![None; inp.edges.len()],
        cross: bc,
        cross0,
        edges: inp.edges.iter().map(|e| (e.a, e.b)).collect(),
    };
    let ids: Vec<String> = inp.edges.iter().map(|e| e.id.clone()).collect();
    smart_route(&mut lay, &ids);
    lay
}
