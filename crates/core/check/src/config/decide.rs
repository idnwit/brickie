//! The one decision whether an edge between two sibling bricks is allowed. `check`, the design
//! editor, the unused-link list and the viewer all call [`decide`].

use super::{Action, Brick, Config, DepRule, Sel};

/// What a level decides for an edge, and the rule that decided it (`None`: no rule applied, so
/// the fixed default for the pair holds).
#[derive(Debug, Clone, Copy)]
pub struct Decision<'a> {
    pub action: Action,
    /// Index into the level's rules, and the rule.
    pub rule: Option<(usize, &'a DepRule)>,
}

/// Selector specificity for a rule side: brick 3, tag 2, `*` 1; `*` never matches an
/// independent brick.
fn rank(sel: &Sel, b: &Brick) -> Option<u32> {
    match sel {
        Sel::Any if b.independent => None,
        s => s.specificity(b),
    }
}

/// The rule of `level` that decides `from -> to` of `kind` / `scope`, entering `to` through
/// child `entered`: highest summed specificity (`via` adds one), deny on a tie.
fn best<'a>(
    level: &'a Config,
    from: &Brick,
    to: &Brick,
    entered: Option<&str>,
    kind: &str,
    scope: &str,
) -> Option<(usize, &'a DepRule)> {
    let mut best: Option<(u32, usize, &DepRule)> = None;
    for (i, r) in level.rules.iter().enumerate() {
        if !(r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind))
            || !r.scope.matches(scope)
            || !r.enters(entered)
        {
            continue;
        }
        let (Some(a), Some(b)) = (rank(&r.from, from), rank(&r.to, to)) else {
            continue;
        };
        // A `via` rule scores one more on the `to` side.
        let rank = a + b + u32::from(r.via.is_some());
        let better = best.is_none_or(|(s, _, cur)| {
            rank > s || (rank == s && r.action == Action::Deny && cur.action == Action::Allow)
        });
        if better {
            best = Some((rank, i, r));
        }
    }
    best.map(|(_, i, r)| (i, r))
}

/// Whether the sibling edge `from -> to` of `kind` (`uses` | `implements` | `reexports`) and
/// `scope` (`prod` | `test`) is allowed at `level`. `entered` is the child of `to` the edge
/// enters through (`None` for a leaf `to` or when unknown); a rule with `via` needs it. With no matching rule an edge is denied,
/// except one into an independent brick, which is allowed. An independent brick may use only
/// another independent brick, so an allow from it to a normal brick never applies.
pub fn decide<'a>(
    level: &'a Config,
    from: &Brick,
    to: &Brick,
    entered: Option<&str>,
    kind: &str,
    scope: &str,
) -> Decision<'a> {
    let rule = best(level, from, to, entered, kind, scope);
    let closed = from.independent && !to.independent;
    match rule {
        Some((_, r)) if closed && r.action == Action::Allow => Decision {
            action: Action::Deny,
            rule: None,
        },
        Some((_, r)) => Decision {
            action: r.action,
            rule,
        },
        None if to.independent && !from.independent => Decision {
            action: Action::Allow,
            rule: None,
        },
        None => Decision {
            action: Action::Deny,
            rule: None,
        },
    }
}
