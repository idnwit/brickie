//! Per-level module edge aggregation: folds module edges into level edges `(from, to, kind)`,
//! intra-group member edges `(group, from_member, to_member, kind)`, and hidden edges into
//! independent targets.

use super::edges::{End, ModEdge, end_unit};
use brickie_check::config::Config;
use std::collections::{BTreeMap, BTreeSet};

/// Leaf paths of outside neighbours.
pub(super) type Paths = BTreeSet<Vec<String>>;

#[derive(Default)]
pub(super) struct Acc {
    pub prod: bool,
    pub refs: usize,
    pub ids: BTreeSet<String>,
}

fn outside_path(x: &End) -> Option<&Vec<String>> {
    match x {
        End::Outside(p) => Some(p),
        End::Child(_) => None,
    }
}

/// The module edges of one level folded by `(from, to, kind)`.
pub(super) struct Agg {
    pub edges: BTreeMap<(End, End, String), Acc>,
    /// Intra-group edges keyed by `(group_name, from_member, to_member, kind)`.
    pub internal: BTreeMap<(String, String, String, String), Acc>,
    /// Per independent target, the ends whose clean edges into it are left out (no ghost, no
    /// edge, no count).
    pub hidden: BTreeMap<End, BTreeSet<End>>,
}

impl Agg {
    /// Edges with a child of `level` at either end; both ends outside are not this level's.
    pub(super) fn new(root: &Config, edges: &[ModEdge], level: &[String]) -> Agg {
        let mut agg: BTreeMap<(End, End, String), Acc> = BTreeMap::new();
        let mut internal: BTreeMap<(String, String, String, String), Acc> = BTreeMap::new();
        let mut hidden: BTreeMap<End, BTreeSet<End>> = BTreeMap::new();
        for e in edges {
            let (Some((a, a_child)), Some((b, b_child))) =
                (end_unit(root, &e.from, level), end_unit(root, &e.to, level))
            else {
                continue;
            };
            let inside = |x: &End| matches!(x, End::Child(_));
            if a == b {
                if let (End::Child(group), Some(fm), Some(tm)) = (&a, a_child, b_child)
                    && fm != tm
                {
                    let acc = internal
                        .entry((group.clone(), fm, tm, e.kind.clone()))
                        .or_default();
                    acc.prod |= e.prod;
                    acc.refs += e.refs;
                    acc.ids.extend(e.diags.iter().cloned());
                }
                continue;
            }
            if !(inside(&a) || inside(&b)) {
                continue;
            }
            if e.hidden {
                hidden.entry(b).or_default().insert(a);
                continue;
            }
            let acc = agg.entry((a, b, e.kind.clone())).or_default();
            acc.prod |= e.prod;
            acc.refs += e.refs;
            acc.ids.extend(e.diags.iter().cloned());
        }
        Agg {
            edges: agg,
            internal,
            hidden,
        }
    }

    /// The outside neighbours: those that are an edge source, and all of them.
    pub(super) fn outside(&self) -> (Paths, Paths) {
        let keys = || self.edges.keys();
        (
            keys()
                .filter_map(|(a, _, _)| outside_path(a).cloned())
                .collect(),
            keys()
                .flat_map(|(a, b, _)| [a, b])
                .filter_map(|x| outside_path(x).cloned())
                .collect(),
        )
    }
}
