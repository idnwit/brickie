//! Rules between two named bricks: the unused allow rules ([`decide`] picks them for no measured
//! edge, whether or not the ends have code yet; design todos, not findings) and every rule as
//! Design mode draws it.

use crate::config::{Action, Config, DepRule, Link, Resolved, Sel, decide};
use brickie_facts::Cache;
use std::collections::BTreeSet;

/// A measured module edge lifted to its level.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Hop {
    path: Vec<String>,
    from: String,
    to: String,
    /// The child of `to` the edge enters through, if `to` is a composite.
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
            && decide(lv, f, t, h.entered.as_deref(), &h.kind, &h.scope)
                .rule
                .is_some_and(|(_, d)| std::ptr::eq(d, r))
    })
}

/// Every unbacked allow rule between named bricks, in tree order, one per rule location.
pub fn unused_links(cache: &Cache, cfg: &Config, res: &Resolved) -> Vec<Link> {
    let hops = lifted(cache, res);
    let mut out = Vec::new();
    walk(cfg, &mut out, &|level, r, from, to| {
        // An allow between a normal and an independent brick never changes a decision.
        let indep = |n: &str| level.brick(n).is_some_and(|b| b.independent);
        indep(to) == indep(from) && r.action == Action::Allow && !backed(level, r, &hops)
    });
    out
}

/// Every rule between named bricks (allow or deny), in tree order, one per rule location:
/// what Design mode draws from the design files alone.
pub fn rule_links(cfg: &Config) -> Vec<Link> {
    let mut out = Vec::new();
    walk(cfg, &mut out, &|_, _, _, _| true);
    out
}

type Keep<'a> = dyn Fn(&Config, &DepRule, &str, &str) -> bool + 'a;

fn walk(level: &Config, out: &mut Vec<Link>, keep: &Keep) {
    for r in &level.rules {
        let (Sel::Brick(from), Sel::Brick(to)) = (&r.from, &r.to) else {
            continue;
        };
        if !keep(level, r, from, to) || out.iter().any(|l| l.loc == r.loc) {
            continue;
        }
        out.push(Link {
            level: level.path.clone(),
            from: from.clone(),
            to: to.clone(),
            via: r.via.clone().unwrap_or_default(),
            kinds: r.kinds.clone(),
            scope: r.scope,
            action: r.action,
            file: level.file.clone(),
            loc: r.loc.clone(),
            reason: r.reason.clone(),
        });
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, out, keep);
        }
    }
}
