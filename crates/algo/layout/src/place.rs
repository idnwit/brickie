//! x placement by weighted isotonic regression (`pava`, `sep`, `place` and the x block of
//! `computeLayout`): priority to long-edge dummies keeps trunks straight.

use super::order::{LNode, Layers};
use super::*;

/// `pava`: nondecreasing `y` minimising `sum w * (y - t)^2` (pool adjacent violators).
pub fn pava(t: &[f64], w: &[f64]) -> Vec<f64> {
    struct B {
        s: f64,
        w: f64,
        n: usize,
    }
    let mut st: Vec<B> = Vec::new();
    for i in 0..t.len() {
        let mut b = B {
            s: w[i] * t[i],
            w: w[i],
            n: 1,
        };
        while st.last().is_some_and(|p| p.s / p.w > b.s / b.w) {
            let p = st.pop().unwrap_or(B {
                s: 0.0,
                w: 1.0,
                n: 0,
            });
            b = B {
                s: p.s + b.s,
                w: p.w + b.w,
                n: p.n + b.n,
            };
        }
        st.push(b);
    }
    let mut y = Vec::with_capacity(t.len());
    for b in &st {
        for _ in 0..b.n {
            y.push(b.s / b.w);
        }
    }
    y
}

/// Which neighbours pull a node in [`place`].
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Up,
    Dn,
    Both,
}

/// `sep`: centre distance minimum of two neighbouring nodes in a row.
fn sep(ns: &[LNode], p: usize, q: usize) -> f64 {
    let (a, b) = (&ns[p], &ns[q]);
    let (wp, wq) = (a.w / 2.0, b.w / 2.0);
    if a.real && b.real {
        return wp + wq + GAP;
    }
    wp + wq + if a.real || b.real { DUM } else { 14.0 }
}

/// `place`: x of every node of row `li`, pulled towards its neighbours (weighted PAVA).
fn place(
    ns: &mut [LNode],
    layers: &Layers,
    wt: &HashMap<(usize, usize), f64>,
    li: usize,
    mode: Mode,
) {
    let row = &layers[li];
    if row.is_empty() {
        return;
    }
    let edge_w = |p: usize, q: usize| {
        wt.get(&(p, q))
            .or_else(|| wt.get(&(q, p)))
            .copied()
            .filter(|v| *v != 0.0)
            .unwrap_or(1.0)
    };
    let con: Vec<usize> = row
        .iter()
        .copied()
        .filter(|&n| !ns[n].up.is_empty() || !ns[n].dn.is_empty())
        .collect();
    let cen = if con.is_empty() {
        None
    } else {
        Some(con.iter().map(|&n| ns[n].x).sum::<f64>() / con.len() as f64)
    };
    let (mut t, mut w) = (Vec::new(), Vec::new());
    for &n in row {
        let nb: Vec<usize> = match mode {
            Mode::Up => ns[n].up.clone(),
            Mode::Dn => ns[n].dn.clone(),
            Mode::Both => ns[n].up.iter().chain(&ns[n].dn).copied().collect(),
        };
        if nb.is_empty() {
            let unconnected = ns[n].up.is_empty() && ns[n].dn.is_empty();
            t.push(match cen {
                Some(c) if unconnected => c,
                _ => ns[n].x,
            });
            w.push(0.01);
            continue;
        }
        let (mut s, mut sw) = (0.0, 0.0);
        for q in nb {
            let ew = edge_w(n, q);
            s += ew * ns[q].x;
            sw += ew;
        }
        t.push(s / sw);
        w.push(sw);
    }
    let mut off = vec![0.0];
    for i in 1..row.len() {
        off.push(off[i - 1] + sep(ns, row[i - 1], row[i]));
    }
    let tt: Vec<f64> = t.iter().zip(&off).map(|(v, o)| v - o).collect();
    let y = pava(&tt, &w);
    for (i, &n) in row.iter().enumerate() {
        ns[n].x = y[i] + off[i];
    }
}

/// x of every node: rows centred, then 10 up/down sweeps and 4 free sweeps of [`place`].
/// Returns the horizontal extent `(min, max)` of all boxes.
pub fn place_x(ns: &mut [LNode], layers: &Layers, wt: &HashMap<(usize, usize), f64>) -> (f64, f64) {
    let n_l = layers.len();
    let mut maxw: f64 = 0.0;
    let widths: Vec<f64> = layers
        .iter()
        .map(|row| {
            let mut w = 0.0;
            for (i, &n) in row.iter().enumerate() {
                w += ns[n].w;
                if i > 0 {
                    w += sep(ns, row[i - 1], n) - ns[row[i - 1]].w / 2.0 - ns[n].w / 2.0;
                }
            }
            maxw = maxw.max(w);
            w
        })
        .collect();
    for (l, row) in layers.iter().enumerate() {
        let mut x = (maxw - widths[l]) / 2.0;
        for (i, &n) in row.iter().enumerate() {
            if i > 0 {
                x += sep(ns, row[i - 1], n) - ns[row[i - 1]].w / 2.0 - ns[n].w / 2.0;
            }
            ns[n].x = x + ns[n].w / 2.0;
            x += ns[n].w;
        }
    }
    for _ in 0..10 {
        for l in 1..n_l {
            place(ns, layers, wt, l, Mode::Up);
        }
        for l in (0..n_l.saturating_sub(1)).rev() {
            place(ns, layers, wt, l, Mode::Dn);
        }
    }
    for _ in 0..4 {
        for l in 0..n_l {
            place(ns, layers, wt, l, Mode::Both);
        }
    }
    let (mut min_x, mut max_x) = (1e9f64, -1e9f64);
    for n in ns.iter() {
        min_x = min_x.min(n.x - n.w / 2.0);
        max_x = max_x.max(n.x + n.w / 2.0);
    }
    if min_x == 1e9 {
        min_x = 0.0;
        max_x = 0.0;
    }
    (min_x, max_x)
}
