//! Rows pinned by layer and the layered graph: one node per brick in its row, dummy nodes for
//! edges that span rows (parallel edges of one pair share a chain = a bundled trunk), same-row
//! edges kept apart as arcs. First part of `computeLayout`.

use super::order::{LNode, Layers};
use super::*;

/// One chain of dummy nodes between a pair of bricks and the edges that share it.
struct Pair {
    chain: Vec<usize>,
    eids: Vec<usize>,
}

/// The layered graph before crossing minimisation.
pub struct Built {
    pub ns: Vec<LNode>,
    pub layers: Layers,
    /// Layer index to input row.
    pub used: Vec<usize>,
    /// Processed (non-self) edges in id order.
    pub es: Vec<usize>,
    /// Same-row edges.
    pub arcs: Vec<usize>,
    /// Chain routes; arcs are decided later.
    pub kinds: Vec<Option<Kind>>,
    /// Placement weights of chain segments (`edgeWt`).
    pub wt: HashMap<(usize, usize), f64>,
}

/// `Wd`: box width from the name length, with room for the score badge in the corner.
fn wd(id: &str) -> f64 {
    (id.chars().count() as f64 * 8.9 + 62.0).clamp(150.0, 276.0)
}

pub fn build(inp: &Input) -> Built {
    let nb = inp.bricks.len();
    let mut used: Vec<usize> = inp.bricks.iter().map(|b| b.row).collect();
    used.sort_unstable();
    used.dedup();
    let n_l = used.len();
    let li_of = |row: usize| used.iter().position(|&r| r == row).unwrap_or(0);

    // Wide enough for every top/bottom port at PORT spacing.
    let (mut d_u, mut d_d) = (vec![0usize; nb], vec![0usize; nb]);
    for e in &inp.edges {
        if e.a == e.b {
            continue;
        }
        let (ra, rb) = (inp.bricks[e.a].row, inp.bricks[e.b].row);
        if ra == rb {
            continue;
        }
        let (up, lo) = if ra < rb { (e.a, e.b) } else { (e.b, e.a) };
        d_d[up] += 1;
        d_u[lo] += 1;
    }
    let mut ns: Vec<LNode> = Vec::new();
    let mut layers: Layers = vec![Vec::new(); n_l];
    for (i, b) in inp.bricks.iter().enumerate() {
        let w = wd(&b.id).max(2.0 * PAD + PORT * (d_u[i].max(d_d[i]) as f64 - 1.0));
        ns.push(LNode {
            id: b.id.clone(),
            real: true,
            li: li_of(b.row),
            w,
            h: H,
            bi: i,
            up: vec![],
            upw: vec![],
            dn: vec![],
            dnw: vec![],
            x: 0.0,
            y: 0.0,
            i: 0,
            key: 0.0,
        });
        layers[ns[i].li].push(i);
    }
    for row in layers.iter_mut() {
        row.sort_by_key(|&n| n); // brick (config) order
        let len = row.len();
        for (i, &n) in row.iter().enumerate() {
            ns[n].i = i;
            ns[n].key = (i as f64 + 0.5) / len as f64;
        }
    }

    // Main chains; parallel edges (same pair) share one dummy chain = a bundled trunk.
    let mut es: Vec<usize> = (0..inp.edges.len())
        .filter(|&e| inp.edges[e].a != inp.edges[e].b)
        .collect();
    es.sort_by(|&p, &q| inp.edges[p].id.cmp(&inp.edges[q].id));
    let mut up_ends: HashMap<usize, End> = HashMap::new();
    let mut kinds: Vec<Option<Kind>> = vec![None; inp.edges.len()];
    let mut arcs: Vec<usize> = Vec::new();
    let mut pairs: Vec<Pair> = Vec::new();
    let mut pair_ix: HashMap<(usize, usize), usize> = HashMap::new();
    let mut nd = 0;
    for &e in &es {
        let (a, b) = (inp.edges[e].a, inp.edges[e].b);
        if ns[a].li == ns[b].li {
            arcs.push(e);
            continue;
        }
        let up_a = ns[a].li < ns[b].li;
        let (u, lo) = if up_a { (a, b) } else { (b, a) };
        let m = ns[lo].li - ns[u].li;
        let pi = match pair_ix.get(&(u, lo)) {
            Some(&p) => p,
            None => {
                let pk = format!("{}>{}", ns[u].id, ns[lo].id);
                let mut chain = vec![u];
                for k in 1..m {
                    let t = k as f64 / m as f64;
                    let idx = ns.len();
                    let li = ns[u].li + k;
                    ns.push(LNode {
                        id: format!("~{nd}:{pk}"),
                        real: false,
                        li,
                        w: 0.0,
                        h: H,
                        bi: 1_000_000_000,
                        up: vec![],
                        upw: vec![],
                        dn: vec![],
                        dnw: vec![],
                        x: 0.0,
                        y: 0.0,
                        i: 0,
                        key: ns[u].key * (1.0 - t) + ns[lo].key * t,
                    });
                    nd += 1;
                    chain.push(idx);
                    layers[li].push(idx);
                }
                chain.push(lo);
                pairs.push(Pair {
                    chain,
                    eids: vec![],
                });
                pair_ix.insert((u, lo), pairs.len() - 1);
                pairs.len() - 1
            }
        };
        pairs[pi].eids.push(e);
        up_ends.insert(e, if up_a { End::A } else { End::B });
    }
    let mut wt: HashMap<(usize, usize), f64> = HashMap::new();
    for p in pairs.iter_mut() {
        p.eids
            .sort_by(|&a, &b| inp.edges[a].id.cmp(&inp.edges[b].id));
        let cnt = p.eids.len();
        for (i, &e) in p.eids.iter().enumerate() {
            kinds[e] = Some(Kind::Chain {
                chain: p.chain.clone(),
                up_end: up_ends[&e],
                idx: i,
                cnt,
            });
        }
        let cw = cnt.min(3) as f64;
        for k in 0..p.chain.len() - 1 {
            let (a, b) = (p.chain[k], p.chain[k + 1]);
            ns[a].dn.push(b);
            ns[b].up.push(a);
            ns[a].dnw.push(cnt as f64);
            ns[b].upw.push(cnt as f64);
            let nr = usize::from(!ns[a].real) + usize::from(!ns[b].real);
            let w = if nr == 2 {
                12.0
            } else if ns[a].real && ns[b].real {
                1.0
            } else {
                4.0
            };
            wt.insert((a, b), w * cw);
        }
        for &d in &p.chain {
            if !ns[d].real {
                ns[d].w = (cnt as f64 - 1.0) * LANE;
            }
        }
    }
    for row in layers.iter_mut() {
        row.sort_by(|&p, &q| {
            ns[p]
                .key
                .total_cmp(&ns[q].key)
                .then(ns[p].id.cmp(&ns[q].id))
        });
        for (i, &n) in row.iter().enumerate() {
            ns[n].i = i;
        }
    }
    Built {
        ns,
        layers,
        used,
        es,
        arcs,
        kinds,
        wt,
    }
}
