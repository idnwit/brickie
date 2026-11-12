//! Module edges resolved to leaf paths, and their aggregation onto the bricks of one level:
//! both ends inside the level map to children, an end outside lifts to the sibling of the
//! nearest enclosing level (the faded neighbour).

use brickie_core::check::Loaded;
use brickie_core::diag;
use brickie_core::rules;
use std::collections::BTreeSet;

/// A module edge with the leaf paths of its ends and the ids of its `deps/*` findings.
pub struct ModEdge {
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub kind: String,
    pub prod: bool,
    pub refs: usize,
    pub diags: Vec<String>,
}

/// Module edges between mapped modules of different leaves.
pub fn mod_edges(l: &Loaded) -> Vec<ModEdge> {
    let found: BTreeSet<&str> = l.diagnostics.iter().map(|d| d.id.as_str()).collect();
    let codes = [rules::DEPS_DENY, rules::DEPS_DEFAULT, rules::DEPS_PRIVATE];
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
            .collect();
        out.push(ModEdge {
            from: from.to_vec(),
            to: to.to_vec(),
            kind: e.kind.clone(),
            prod: e.scope == "prod",
            refs: e.evidence.len(),
            diags,
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

/// Where leaf path `p` lands seen from level `level`; `None` for a path that is the level or
/// above it (cannot happen for leaves of a composite level).
pub fn end(p: &[String], level: &[String]) -> Option<End> {
    if p.starts_with(level) {
        return p.get(level.len()).map(|c| End::Child(c.clone()));
    }
    let k = p.iter().zip(level).take_while(|(a, b)| a == b).count();
    p.get(..k + 1).map(|q| End::Outside(q.to_vec()))
}
