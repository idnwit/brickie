//! The unused allow rules between two named bricks: no measured edge matches them, whether or not
//! the ends have code yet (reported as `deps/unused`). Rules backed by a measured edge are not
//! listed.

use crate::config::{Action, Config, DepRule, Resolved, Scope, Sel};
use brickie_facts::Cache;
use std::collections::BTreeSet;

/// An allow rule between two named bricks that no measured edge backs.
#[derive(Debug, Clone)]
pub struct Link {
    /// Names from the root to the level the rule is declared in.
    pub level: Vec<String>,
    pub from: String,
    pub to: String,
    pub via: Option<Vec<String>>,
    /// Empty means all kinds.
    pub kinds: Vec<String>,
    pub scope: Scope,
    /// The config file of the level and `file:line` of the rule.
    pub file: String,
    pub loc: String,
    pub reason: Option<String>,
}

/// A measured module edge lifted to its level.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Hop {
    path: Vec<String>,
    from: String,
    to: String,
    /// The child of `to` the edge enters through (`None` for a leaf).
    entered: Option<String>,
    kind: String,
    scope: String,
}

fn lifted(cache: &Cache, res: &Resolved) -> BTreeSet<Hop> {
    let mut out = BTreeSet::new();
    for e in cache.edges() {
        let (Some(p1), Some(p2)) = (res.path(&e.from), res.path(&e.to)) else {
            continue;
        };
        let k = p1.iter().zip(p2).take_while(|(a, b)| a == b).count();
        if let (Some(a), Some(b)) = (p1.get(k), p2.get(k)) {
            out.insert(Hop {
                path: p1[..k].to_vec(),
                from: a.clone(),
                to: b.clone(),
                entered: p2.get(k + 1).cloned(),
                kind: e.kind,
                scope: e.scope,
            });
        }
    }
    out
}

fn backed(lv: &Config, r: &DepRule, measured: &BTreeSet<Hop>) -> bool {
    measured.iter().any(|h| {
        let (Some(f), Some(t)) = (lv.brick(&h.from), lv.brick(&h.to)) else {
            return false;
        };
        h.path == lv.path
            && r.from.specificity(f).is_some()
            && r.to_rank(t, h.entered.as_deref()).is_some()
            && (r.kinds.is_empty() || r.kinds.contains(&h.kind))
            && r.scope.matches(&h.scope)
    })
}

/// Every unbacked allow rule between named bricks, in tree order, one per rule location.
pub fn unused_links(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<Link> {
    let hops = lifted(cache, res);
    let mut out = Vec::new();
    walk(cfg, &hops, &mut out);
    out
}

fn walk(level: &Config, hops: &BTreeSet<Hop>, out: &mut Vec<Link>) {
    for r in &level.deps.rules {
        let (Sel::Brick(from), Sel::Brick(to)) = (&r.from, &r.to) else {
            continue;
        };
        if r.action != Action::Allow || backed(level, r, hops) || out.iter().any(|l| l.loc == r.loc)
        {
            continue;
        }
        out.push(Link {
            level: level.path.clone(),
            from: from.clone(),
            to: to.clone(),
            via: r.via.clone(),
            kinds: r.kinds.clone(),
            scope: r.scope,
            file: level.file.clone(),
            loc: r.loc.clone(),
            reason: r.reason.clone(),
        });
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, hops, out);
        }
    }
}
