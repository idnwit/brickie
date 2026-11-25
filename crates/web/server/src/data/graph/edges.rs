//! Module edges resolved to leaf paths, and their aggregation onto the bricks of one level:
//! both ends inside the level map to children, an end outside lifts to the sibling of the
//! nearest enclosing level (the faded neighbour).

use brickie_check::check as rules;
use brickie_check::check::Loaded;
use brickie_check::config::Config;
use brickie_check::diag;
use std::collections::BTreeSet;

/// A module edge with the leaf paths of its ends and the ids of its `deps/*` findings.
pub struct ModEdge {
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub kind: String,
    pub prod: bool,
    pub refs: usize,
    pub diags: Vec<String>,
    /// A clean edge into an independent lib from a non-independent brick: hidden in the view.
    pub hidden: bool,
}

/// Module edges between mapped modules of different leaves.
pub fn mod_edges(l: &Loaded) -> Vec<ModEdge> {
    let found: BTreeSet<&str> = l.diagnostics.iter().map(|d| d.id.as_str()).collect();
    let codes = [rules::DEPS_UNLINKED, rules::DEPS_PRIVATE];
    // An edge from inside the parent's subtree into an independent lib at any depth, from
    // outside that lib.
    let into_lib = |from: &[String], to: &[String]| {
        (1..=to.len()).any(|k| {
            from.len() >= k
                && from[..k - 1] == to[..k - 1]
                && from[k - 1] != to[k - 1]
                && l.cfg.brick_at(&to[..k]).is_some_and(|b| b.independent)
        })
    };
    let mut out = Vec::new();
    for e in l.cache.edges() {
        let (Some(from), Some(to)) = (l.resolved.path(&e.from), l.resolved.path(&e.to)) else {
            continue;
        };
        if from == to {
            continue;
        }
        let subject = rules::dep_subject(&e.from, &e.to, &e.kind, &e.scope);
        let diags = codes
            .iter()
            .map(|c| diag::finding_id(c, &subject))
            .filter(|id| found.contains(id.as_str()))
            .collect::<Vec<String>>();
        let hidden = into_lib(from, to) && diags.is_empty();
        out.push(ModEdge {
            from: from.to_vec(),
            to: to.to_vec(),
            kind: e.kind.clone(),
            prod: e.scope == "prod",
            refs: e.evidence.len(),
            diags,
            hidden,
        });
    }
    out
}

/// One end of an edge seen from a level.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum End {
    /// A child of the level, by name.
    Child(String),
    /// A brick outside the level: the path lifted to the sibling of the nearest enclosing
    /// level that contains it.
    Outside(Vec<String>),
}

/// Where leaf path `p` lands seen from level `level`, lifted to logical units (a grouped brick
/// lifts to its enclosing group both inside `level` and in any ancestor level). Returns the
/// lifted [`End`] and, for a child of `level`, the raw child brick name before group lifting.
pub fn end_unit(root: &Config, p: &[String], level: &[String]) -> Option<(End, Option<String>)> {
    if p.starts_with(level) {
        let child = p.get(level.len())?;
        let unit = root
            .node(level)
            .and_then(|c| c.brick(child).map(|b| c.unit_of(b).name.to_string()))
            .unwrap_or_else(|| child.clone());
        return Some((End::Child(unit), Some(child.clone())));
    }
    let k = p.iter().zip(level).take_while(|(a, b)| a == b).count();
    let q = p.get(..k + 1)?;
    let mut lifted = q.to_vec();
    if let Some(anc) = root.node(&q[..k])
        && let Some(b) = anc.brick(&q[k])
    {
        lifted[k] = anc.unit_of(b).name.to_string();
    }
    Some((End::Outside(lifted), None))
}
