//! Crossing minimisation: row orders with the fewest weighted crossings. Port of the original's
//! `layerCross` .. `annealOrder`, `sweep` and the restart loop of `computeLayout`.

use super::anneal::{Lcg, anneal_order, set_order};
use super::{ITER, RESTARTS};

/// The layered-graph node of the original (`n` objects): bricks first (index = brick index),
/// then dummies of long edges.
#[derive(Debug, Clone)]
pub struct LNode {
    pub id: String,
    pub real: bool,
    pub li: usize,
    pub w: f64,
    pub h: f64,
    /// Brick index (config order); `1e9` for dummies.
    pub bi: usize,
    pub up: Vec<usize>,
    pub upw: Vec<f64>,
    pub dn: Vec<usize>,
    pub dnw: Vec<f64>,
    pub x: f64,
    pub y: f64,
    /// Position in the row.
    pub i: usize,
    pub key: f64,
}

pub type Layers = Vec<Vec<usize>>;

/// `layerCross`: weighted crossings between row `u` and the row `d` below (BIT counting).
pub fn layer_cross(ns: &[LNode], u: &[usize], d: &[usize]) -> f64 {
    let mut es: Vec<(usize, usize, f64)> = Vec::new();
    for &a in u {
        for (k, &b) in ns[a].dn.iter().enumerate() {
            es.push((ns[a].i, ns[b].i, ns[a].dnw[k]));
        }
    }
    if es.len() < 2 {
        return 0.0;
    }
    es.sort_by(|p, q| p.0.cmp(&q.0).then(p.1.cmp(&q.1)));
    let n_l = d.len();
    let mut bit = vec![0.0; n_l + 2];
    let (mut c, mut tot) = (0.0, 0.0);
    for &(_, v, w) in &es {
        let mut s = 0.0;
        let mut k = v + 1;
        while k > 0 {
            s += bit[k];
            k -= k & k.wrapping_neg();
        }
        c += w * (tot - s);
        tot += w;
        let mut k = v + 1;
        while k <= n_l {
            bit[k] += w;
            k += k & k.wrapping_neg();
        }
    }
    c
}

/// `totalCross` (the soft family penalty is dropped: a row holds one tag).
pub fn total_cross(ns: &[LNode], layers: &Layers) -> f64 {
    (0..layers.len().saturating_sub(1))
        .map(|l| layer_cross(ns, &layers[l], &layers[l + 1]))
        .sum()
}

/// `pc`: weighted crossings between two neighbour lists.
fn pc(ns: &[LNode], a: &[usize], aw: &[f64], b: &[usize], bw: &[f64]) -> f64 {
    let mut c = 0.0;
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            if ns[x].i > ns[y].i {
                c += aw[i] * bw[j];
            }
        }
    }
    c
}

/// `pairCost`: weighted crossings of u's and v's edges (both row pairs) when u is left of v.
pub fn pair_cost(ns: &[LNode], u: usize, v: usize) -> f64 {
    let (a, b) = (&ns[u], &ns[v]);
    pc(ns, &a.up, &a.upw, &b.up, &b.upw) + pc(ns, &a.dn, &a.dnw, &b.dn, &b.dnw)
}

/// `transposeAll`: adjacent swaps that lower weighted crossings.
pub fn transpose_all(ns: &mut [LNode], layers: &mut Layers) {
    let mut pass = 0;
    let mut imp = true;
    while imp && pass < 8 {
        imp = false;
        for l in 0..layers.len() {
            let len = layers[l].len();
            for i in 0..len.saturating_sub(1) {
                let (u, v) = (layers[l][i], layers[l][i + 1]);
                let d0 = pair_cost(ns, u, v);
                let d1 = pair_cost(ns, v, u);
                if d1 < d0 - 1e-9 {
                    layers[l][i] = v;
                    layers[l][i + 1] = u;
                    ns[v].i = i;
                    ns[u].i = i + 1;
                    imp = true;
                }
            }
        }
        pass += 1;
    }
}

/// `siftRow`: each node tries every slot of its row and keeps the cheapest.
pub fn sift_row(ns: &mut [LNode], row: &mut Vec<usize>) -> bool {
    let mut imp = false;
    let mut nodes = row.clone();
    nodes.sort_by(|&p, &q| {
        (ns[q].up.len() + ns[q].dn.len())
            .cmp(&(ns[p].up.len() + ns[p].dn.len()))
            .then(ns[p].id.cmp(&ns[q].id))
    });
    for n in nodes {
        let i = row.iter().position(|&x| x == n).unwrap_or(0);
        row.remove(i);
        let ab: Vec<f64> = row.iter().map(|&m| pair_cost(ns, m, n)).collect();
        let ba: Vec<f64> = row.iter().map(|&m| pair_cost(ns, n, m)).collect();
        let mut c: f64 = ba.iter().sum();
        let (mut best_i, mut best_c) = (0, f64::INFINITY);
        for j in 0..=row.len() {
            if j > 0 {
                c += ab[j - 1] - ba[j - 1];
            }
            if c < best_c - 1e-9 {
                best_c = c;
                best_i = j;
            }
        }
        row.insert(best_i, n);
        if best_i != i {
            imp = true;
        }
        for (k, &m) in row.iter().enumerate() {
            ns[m].i = k;
        }
    }
    imp
}

/// `siftAll`.
pub fn sift_all(ns: &mut [LNode], layers: &mut Layers) -> bool {
    let mut imp = false;
    for l in 0..layers.len() {
        let mut row = std::mem::take(&mut layers[l]);
        if row.len() > 1 && sift_row(ns, &mut row) {
            imp = true;
        }
        layers[l] = row;
    }
    imp
}

/// `sweep` (inside `computeLayout`): reorder row `li` by the barycentre or median of its
/// neighbours in the row above (`up`) or below.
pub fn sweep(ns: &mut [LNode], layers: &mut Layers, li: usize, up: bool, med: bool) {
    let nl = if up { li.checked_sub(1) } else { Some(li + 1) };
    let Some(nl) = nl.filter(|&n| n < layers.len()) else {
        return;
    };
    let _ = nl;
    let row = layers[li].clone();
    let mut val: Vec<Option<f64>> = vec![None; row.len()];
    for (k, &n) in row.iter().enumerate() {
        let (nb, nw) = if up {
            (&ns[n].up, &ns[n].upw)
        } else {
            (&ns[n].dn, &ns[n].dnw)
        };
        if nb.is_empty() {
            continue;
        }
        let mut v: Vec<f64> = Vec::new();
        for (j, &x) in nb.iter().enumerate() {
            for _ in 0..nw[j] as usize {
                v.push(ns[x].i as f64);
            }
        }
        v.sort_by(|a, b| a.total_cmp(b));
        let r = if med {
            let h = v.len() >> 1;
            if v.len() % 2 == 1 {
                v[h]
            } else {
                (v[h - 1] + v[h]) / 2.0
            }
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        };
        val[k] = Some(r);
    }
    let slots: Vec<usize> = (0..row.len()).filter(|&i| val[i].is_some()).collect();
    let mut mv: Vec<usize> = slots.clone();
    // sort moved nodes by value, ties by `fam` (brick index, then id)
    mv.sort_by(|&p, &q| {
        let (a, b) = (row[p], row[q]);
        val[p]
            .unwrap_or(0.0)
            .total_cmp(&val[q].unwrap_or(0.0))
            .then(ns[a].bi.cmp(&ns[b].bi))
            .then(ns[a].id.cmp(&ns[b].id))
    });
    let moved: Vec<usize> = mv.iter().map(|&i| row[i]).collect();
    let mut out = row;
    for (k, &s) in slots.iter().enumerate() {
        out[s] = moved[k];
    }
    set_order(ns, &out);
    layers[li] = out;
}

/// The restart loop of `computeLayout`: returns the best order found (`best`) and its crossings.
pub fn minimise(ns: &mut [LNode], layers: &mut Layers, n_real: usize) -> f64 {
    let nl = layers.len();
    let mut best = layers.clone();
    let mut bc = if nl > 0 { total_cross(ns, layers) } else { 0.0 };
    macro_rules! keep_best {
        () => {{
            let c = total_cross(ns, layers);
            if c < bc - 1e-9 {
                bc = c;
                best = layers.clone();
            }
        }};
    }
    let restarts = if n_real <= 40 { RESTARTS * 2 } else { RESTARTS };
    for r in 0..restarts {
        let mut rnd = Lcg(12345 + r as u64 * 7919);
        for l in 0..layers.len() {
            let row = layers[l].clone();
            let jit: Vec<f64> = row
                .iter()
                .map(|_| {
                    if r > 0 {
                        (rnd.next() - 0.5) * if r < 3 { 0.6 } else { 2.0 }
                    } else {
                        0.0
                    }
                })
                .collect();
            let mut idx: Vec<usize> = (0..row.len()).collect();
            idx.sort_by(|&p, &q| {
                (ns[row[p]].key + jit[p])
                    .total_cmp(&(ns[row[q]].key + jit[q]))
                    .then(ns[row[p]].id.cmp(&ns[row[q]].id))
            });
            let sorted: Vec<usize> = idx.iter().map(|&i| row[i]).collect();
            set_order(ns, &sorted);
            layers[l] = sorted;
        }
        keep_best!();
        if nl > 1 {
            for it in 0..ITER {
                let med = it % 2 == 1;
                for l in 1..nl {
                    sweep(ns, layers, l, true, med);
                }
                transpose_all(ns, layers);
                keep_best!();
                for l in (0..nl - 1).rev() {
                    sweep(ns, layers, l, false, med);
                }
                transpose_all(ns, layers);
                keep_best!();
            }
        }
        let mut k = 0;
        while k < 4 && sift_all(ns, layers) {
            transpose_all(ns, layers);
            keep_best!();
            k += 1;
        }
        keep_best!();
    }
    for (l, row) in best.clone().into_iter().enumerate() {
        set_order(ns, &row);
        layers[l] = row;
    }
    let mut k = 0;
    while k < 6 && sift_all(ns, layers) {
        transpose_all(ns, layers);
        k += 1;
    }
    keep_best!();
    if n_real <= 40 {
        for r in 0..3u64 {
            for (l, row) in best.clone().into_iter().enumerate() {
                set_order(ns, &row);
                layers[l] = row;
            }
            anneal_order(ns, layers, 999 + r * 31);
            sift_all(ns, layers);
            transpose_all(ns, layers);
            keep_best!();
        }
    }
    for (l, row) in best.into_iter().enumerate() {
        set_order(ns, &row);
        layers[l] = row;
    }
    bc
}
