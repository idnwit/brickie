//! Allow rules between two named bricks that no measured module edge uses yet: the code the
//! design asks for and nobody wrote.

use brickie_check::check::Loaded;
use brickie_check::config::{Action, Config, DepRule, Sel};
use brickie_model::codes::DESIGN_PLANNED_EDGE;
use std::collections::BTreeSet;

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

type Lifted = BTreeSet<Hop>;

fn lifted(l: &Loaded) -> Lifted {
    let mut out = Lifted::new();
    for e in l.cache.edges() {
        let (Some(p1), Some(p2)) = (l.resolved.path(&e.from), l.resolved.path(&e.to)) else {
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

fn backed(lv: &Config, r: &DepRule, measured: &Lifted) -> bool {
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

/// `loc` (`file:line`) of each unbacked allow rule between named bricks, and its task text.
pub fn unbacked(l: &Loaded) -> Vec<(String, String)> {
    let measured = lifted(l);
    let mut out = Vec::new();
    walk(&l.cfg, &measured, &mut out);
    out
}

fn walk(level: &Config, measured: &Lifted, out: &mut Vec<(String, String)>) {
    for r in &level.deps.rules {
        let named = matches!((&r.from, &r.to), (Sel::Brick(_), Sel::Brick(_)));
        if named && r.action == Action::Allow && !backed(level, r, measured) {
            let text = format!(
                "{DESIGN_PLANNED_EDGE}: rule {} -> {} in {} allows an edge no measured code uses yet",
                r.from,
                r.to,
                level.level()
            );
            if !out.iter().any(|(loc, _)| *loc == r.loc) {
                out.push((r.loc.clone(), text));
            }
        }
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, measured, out);
        }
    }
}
