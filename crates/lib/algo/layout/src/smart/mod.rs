//! `smartRoute`: every edge starts with the guaranteed box-free layered route (dummy chain or arc)
//! and may switch to one direct cubic between any pair of box sides. A greedy pass (short edges
//! first, then a second pass against all others) picks per edge the candidate with fewest box
//! hits, then fewest crossings with the edges chosen so far, then side preference and length.
//! Ports on one side are then spread evenly, sorted by where the edge goes; a direct route that
//! hits a box after spreading falls back to its layered route.
//!
//! The phases live in sibling files: `cand` (candidate routes), `choose` (cost and greedy
//! choice), `ports` (port spreading and the repair rounds).

mod cand;
mod choose;
mod ports;

use super::path::route_path;
use super::*;
use cand::{Cand, candidates};

/// Read-only context shared by the routing phases.
///
/// Contract: `pos`, `edges` and `ids` are indexed like the layout (`ids[e]` is the unique id of
/// edge `e`, used only for tie-breaks); `eids` lists, in ascending order, the edges that are
/// routed (those with a route kind that are not self edges). `cands`, `choice` and port maps
/// passed to its methods are indexed by edge, and only `eids` entries are meaningful.
pub(super) struct Router<'a> {
    pub(super) pos: &'a [Rect],
    pub(super) edges: &'a [(usize, usize)],
    pub(super) eids: &'a [usize],
    pub(super) ids: &'a [String],
}

/// `smartRoute`: pick a direct cubic or the layered route per edge, then spread the ports.
pub fn smart_route(l: &mut Layout, ids: &[String]) {
    let pos = l.pos.clone();
    let edges = l.edges.clone();
    let eids: Vec<usize> = (0..edges.len())
        .filter(|&e| l.kinds[e].is_some() && edges[e].0 != edges[e].1)
        .collect();
    let old_kind = l.kinds.clone();
    let r = Router {
        pos: &pos,
        edges: &edges,
        eids: &eids,
        ids,
    };
    let mut cands: Vec<Vec<Cand>> = (0..edges.len())
        .map(|e| match eids.binary_search(&e) {
            Ok(_) => candidates(l, e),
            Err(_) => Vec::new(),
        })
        .collect();
    let order = r.order(&cands);
    let mut choice = r.greedy(&cands, &order);
    r.settle(&mut cands, &mut choice);
    let po = r.spread_ports(&cands, &choice);
    for &e in &eids {
        l.kinds[e] = if cands[e][choice[e]].old {
            old_kind[e].clone()
        } else {
            Some(Kind::Bez)
        };
    }
    l.po = po;
    for e in 0..edges.len() {
        l.routes[e] = route_path(l, e, &pos);
    }
}
