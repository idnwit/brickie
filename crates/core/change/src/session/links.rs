//! The links a brick may draw to its siblings, and whether a new rule would change anything
//! (the editor refuses a drop that is already decided that way).

use super::Change;
use anyhow::{Context, Result, bail};
use brickie_check::config::{Action, Brick, Config, DepRule, KINDS, Scope, Sel};
use brickie_model::change::{LinkCheck, LinkOption};

/// The rule deciding an edge `from -> to` of `kind` / `scope` at a level (highest
/// specificity, deny on a tie), or `None` for `deps.default`.
fn decide<'a>(
    level: &'a Config,
    from: &Brick,
    to: &Brick,
    kind: &str,
    scope: &str,
) -> Option<&'a DepRule> {
    let mut best: Option<(u32, &DepRule)> = None;
    for r in &level.deps.rules {
        let kinds_ok = r.kinds.is_empty() || r.kinds.iter().any(|k| k == kind);
        let (Some(a), Some(b)) = (r.from.specificity(from), r.to.specificity(to)) else {
            continue;
        };
        if !kinds_ok || !r.scope.matches(scope) {
            continue;
        }
        let better = match best {
            None => true,
            Some((s, cur)) => {
                a + b > s || (a + b == s && r.action == Action::Deny && cur.action == Action::Allow)
            }
        };
        if better {
            best = Some((a + b, r));
        }
    }
    best.map(|(_, r)| r)
}

fn past(action: &str) -> &'static str {
    if action == "allow" {
        "allowed"
    } else {
        "denied"
    }
}

fn action(a: Action) -> &'static str {
    if a == Action::Allow { "allow" } else { "deny" }
}

/// The bricks of `level` a selector names; an error for a name or tag the level lacks.
fn matching<'a>(level: &'a Config, s: &str) -> Result<Vec<&'a Brick>> {
    let sel = Sel::try_from(s.to_string()).map_err(anyhow::Error::msg)?;
    let out: Vec<&Brick> = (level.bricks.iter())
        .filter(|b| sel.specificity(b).is_some())
        .collect();
    if out.is_empty() {
        bail!("'{s}' names no brick of level {}", level.level());
    }
    Ok(out)
}

/// The rule as asked for: `from -> to`, `kinds` (empty: all), `scope` and `action`.
pub struct Link<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub kinds: &'a [String],
    /// `prod`, `test` or `all`.
    pub scope: &'a str,
    /// `allow` or `deny`.
    pub action: &'a str,
}

fn same_rule(r: &DepRule, l: &Link) -> bool {
    let mut a = r.kinds.clone();
    let mut b = l.kinds.to_vec();
    a.sort();
    b.sort();
    let scope = match r.scope {
        Scope::Prod => "prod",
        Scope::Test => "test",
        Scope::All => "all",
    };
    r.from.to_string() == l.from
        && r.to.to_string() == l.to
        && a == b
        && scope == l.scope
        && action(r.action) == l.action
}

impl Change {
    /// For the brick at `path`, every link it may draw to a sibling: each edge kind and scope
    /// with what the level's rules decide now.
    pub fn link_options(&self, path: &[String]) -> Result<Vec<LinkOption>> {
        let cfg = self.design()?;
        let (_, parent) = path.split_last().context("empty brick path")?;
        let level = cfg.node(parent).context("no such level")?;
        let from = cfg
            .brick_at(path)
            .with_context(|| format!("no brick {path:?}"))?;
        let mut out = Vec::new();
        for to in level.bricks.iter().filter(|b| b.name != from.name) {
            for kind in KINDS {
                for scope in ["prod", "test"] {
                    let r = decide(level, from, to, kind, scope);
                    out.push(LinkOption {
                        to: to.name.clone(),
                        kind: kind.into(),
                        scope: scope.into(),
                        action: action(r.map_or(level.deps.default, |r| r.action)).into(),
                        rule: r.map(|r| r.loc.clone()),
                    });
                }
            }
        }
        Ok(out)
    }

    /// Whether adding `link` at `level` (brick names from the root; `[]`: the root) would
    /// change what the level decides. Redundant when an equal rule exists, or when every edge
    /// it covers (each matching pair, kind and scope) is already decided as `link.action`.
    pub fn link_check(&self, level: &[String], link: &Link) -> Result<LinkCheck> {
        if !matches!(link.action, "allow" | "deny") {
            bail!("action must be allow or deny, not '{}'", link.action);
        }
        let scopes: &[&str] = match link.scope {
            "prod" => &["prod"],
            "test" => &["test"],
            "all" => &["prod", "test"],
            s => bail!("scope must be prod, test or all, not '{s}'"),
        };
        let cfg = self.design()?;
        let lv = cfg
            .node(level)
            .with_context(|| format!("no level {level:?}"))?;
        if let Some(r) = lv.deps.rules.iter().find(|r| same_rule(r, link)) {
            return Ok(LinkCheck {
                redundant: true,
                why: format!("the same rule exists at {}", r.loc),
                rule: Some(r.loc.clone()),
            });
        }
        let (froms, tos) = (matching(lv, link.from)?, matching(lv, link.to)?);
        let kinds: Vec<&str> = match link.kinds.is_empty() {
            true => KINDS.to_vec(),
            false => link.kinds.iter().map(String::as_str).collect(),
        };
        let (mut total, mut changed) = (0, 0);
        let mut deciders = std::collections::BTreeSet::new();
        for f in &froms {
            for t in tos.iter().filter(|t| t.name != f.name) {
                for (k, s) in kinds
                    .iter()
                    .flat_map(|k| scopes.iter().map(move |s| (k, s)))
                {
                    let r = decide(lv, f, t, k, s);
                    total += 1;
                    if action(r.map_or(lv.deps.default, |r| r.action)) != link.action {
                        changed += 1;
                    }
                    deciders
                        .insert(r.map_or_else(|| "deps.default".to_string(), |r| r.loc.clone()));
                }
            }
        }
        if changed > 0 {
            let why = format!("would {} {changed} of {total} edges", link.action);
            return Ok(LinkCheck {
                redundant: false,
                why,
                rule: None,
            });
        }
        let one = (deciders.len() == 1)
            .then(|| deciders.pop_first())
            .flatten();
        let by = one.clone().unwrap_or_else(|| "several rules".into());
        Ok(LinkCheck {
            redundant: true,
            why: format!("already {} by {by}", past(link.action)),
            rule: one.filter(|d| d != "deps.default"),
        })
    }
}
