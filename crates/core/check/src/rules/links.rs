//! The state of every allow rule between two named bricks: the design asks for an edge that no
//! measured code has yet (`Planned`), or that nothing will have because both ends are fully
//! measured (`Unused`, reported as `deps/unused`). Rules that are backed by a measured edge
//! are not listed.

use crate::config::{Action, Config, DepRule, Resolved, Scope, Sel};
use brickie_facts::Cache;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkState {
    /// An end, or a descendant of an end, has no measured module yet.
    Planned,
    /// Both ends are fully measured and no edge matches the rule.
    Unused,
}

impl LinkState {
    pub fn name(self) -> &'static str {
        match self {
            LinkState::Planned => "planned",
            LinkState::Unused => "unused",
        }
    }
}

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
    pub state: LinkState,
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

/// Whether the brick `lv.path + name` has leaves, and every leaf below it has a module.
fn measured(lv: &Config, name: &str, leaves: &BTreeSet<&Vec<String>>) -> bool {
    let Some(b) = lv.brick(name) else {
        return false;
    };
    match &b.sub {
        None => {
            let mut path = lv.path.clone();
            path.push(name.to_string());
            leaves.contains(&path)
        }
        Some(sub) => {
            !sub.bricks.is_empty() && sub.bricks.iter().all(|c| measured(sub, &c.name, leaves))
        }
    }
}

/// Every unbacked allow rule between named bricks, in tree order, one per rule location.
pub fn link_states(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<Link> {
    let hops = lifted(cache, res);
    let leaves: BTreeSet<&Vec<String>> = res.paths.values().collect();
    let mut out = Vec::new();
    walk(cfg, &hops, &leaves, &mut out);
    out
}

fn walk(
    level: &Config,
    hops: &BTreeSet<Hop>,
    leaves: &BTreeSet<&Vec<String>>,
    out: &mut Vec<Link>,
) {
    for r in &level.deps.rules {
        let (Sel::Brick(from), Sel::Brick(to)) = (&r.from, &r.to) else {
            continue;
        };
        if r.action != Action::Allow || backed(level, r, hops) || out.iter().any(|l| l.loc == r.loc)
        {
            continue;
        }
        let full = measured(level, from, leaves) && measured(level, to, leaves);
        out.push(Link {
            level: level.path.clone(),
            from: from.clone(),
            to: to.clone(),
            via: r.via.clone(),
            kinds: r.kinds.clone(),
            scope: r.scope,
            state: if full {
                LinkState::Unused
            } else {
                LinkState::Planned
            },
            file: level.file.clone(),
            loc: r.loc.clone(),
            reason: r.reason.clone(),
        });
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, hops, leaves, out);
        }
    }
}
