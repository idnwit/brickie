//! Simulated annealing on per-row permutations for small graphs (`annealOrder`), with the
//! original's fixed-seed LCG so layouts are deterministic.

use super::order::{LNode, Layers, pair_cost, total_cross};

/// The original's inline LCG (`seed=(seed*1103515245+12345)%2147483648`), in exact integers.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> f64 {
        self.0 = (self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345)) % 2_147_483_648;
        self.0 as f64 / 2_147_483_648.0
    }
}

pub fn set_order(ns: &mut [LNode], row: &[usize]) {
    for (i, &n) in row.iter().enumerate() {
        ns[n].i = i;
    }
}

/// `annealOrder`: simulated annealing on per-row permutations for small graphs.
pub fn anneal_order(ns: &mut [LNode], layers: &mut Layers, seed0: u64) {
    let mut rnd = Lcg(seed0);
    let rows: Vec<usize> = (0..layers.len()).filter(|&l| layers[l].len() > 1).collect();
    if rows.is_empty() {
        return;
    }
    let mut cur = total_cross(ns, layers);
    let mut best = cur;
    let mut snap = layers.clone();
    const N: usize = 20000;
    for it in 0..N {
        let t = 1.5 * (1.0 - it as f64 / N as f64) + 0.03;
        let l = rows[(rnd.next() * rows.len() as f64) as usize];
        let len = layers[l].len();
        let i = (rnd.next() * len as f64) as usize;
        let sgn = if rnd.next() < 0.5 { -1i64 } else { 1 };
        let j = i as i64 + sgn * (1 + (rnd.next() * 3.0) as i64);
        if j < 0 || j >= len as i64 {
            continue;
        }
        let j = j as usize;
        let n = layers[l][i];
        let mut d = 0.0;
        if j > i {
            for k in i + 1..=j {
                let m = layers[l][k];
                d += pair_cost(ns, m, n) - pair_cost(ns, n, m);
            }
        } else {
            for k in j..i {
                let m = layers[l][k];
                d += pair_cost(ns, n, m) - pair_cost(ns, m, n);
            }
        }
        if d <= 0.0 || rnd.next() < (-d / t).exp() {
            let row = &mut layers[l];
            row.remove(i);
            row.insert(j, n);
            let row = row.clone();
            set_order(ns, &row);
            cur += d;
            if cur < best - 1e-9 {
                best = cur;
                snap = layers.clone();
            }
        }
    }
    for (l, row) in snap.into_iter().enumerate() {
        set_order(ns, &row);
        layers[l] = row;
    }
}
