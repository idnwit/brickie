//! Initial ports: edges leave and enter box sides in the order of where they go, spread evenly
//! per side (`pl`, `addp`, `po` of `computeLayout`). `smart_route` re-assigns them later.

use super::order::LNode;
use super::*;

pub fn initial_ports(
    inp: &Input,
    ns: &[LNode],
    kinds: &[Option<Kind>],
    es: &[usize],
) -> HashMap<(usize, End), Port> {
    let mut pl: HashMap<(usize, Side), Vec<(f64, usize, End)>> = HashMap::new();
    let mut addp = |n: usize, sd: Side, e: usize, end: End, ox: f64| {
        pl.entry((n, sd)).or_default().push((ox, e, end));
    };
    for &e in es {
        let (a, b) = (inp.edges[e].a, inp.edges[e].b);
        match &kinds[e] {
            Some(Kind::Arc { top, .. }) => {
                let sd = if *top { Side::T } else { Side::B };
                addp(a, sd, e, End::A, ns[b].x);
                addp(b, sd, e, End::B, ns[a].x);
            }
            Some(Kind::Chain { chain, up_end, .. }) => {
                let (u, l) = (chain[0], chain[chain.len() - 1]);
                addp(u, Side::B, e, *up_end, ns[chain[1]].x);
                addp(l, Side::T, e, up_end.other(), ns[chain[chain.len() - 2]].x);
            }
            _ => {}
        }
    }
    let mut po: HashMap<(usize, End), Port> = HashMap::new();
    for ((n, sd), mut list) in pl {
        list.sort_by(|p, q| {
            p.0.total_cmp(&q.0)
                .then(inp.edges[p.1].id.cmp(&inp.edges[q.1].id))
                .then((p.2 == End::B).cmp(&(q.2 == End::B)))
        });
        let hz = matches!(sd, Side::T | Side::B);
        let len = list.len() as f64;
        let st = (if hz { 14.0f64 } else { 3.0 }).min(
            ((if hz { ns[n].w } else { ns[n].h }) - if hz { 20.0 } else { 14.0 })
                / (len - 1.0).max(1.0),
        );
        for (i, p) in list.iter().enumerate() {
            po.insert(
                (p.1, p.2),
                Port {
                    side: sd,
                    off: (i as f64 - (len - 1.0) / 2.0) * st,
                },
            );
        }
    }

    po
}
