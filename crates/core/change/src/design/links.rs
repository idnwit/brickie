//! The links a brick may draw to its siblings, and whether a new rule would change anything
//! (the editor refuses a drop that is already decided that way). Both ask `check::decide`.

use super::Design;
use anyhow::{Context, Result, bail};
use brickie_check::config::{Action, Brick, Config, DepRule, KINDS, Scope, Sel, decide};
use brickie_model::design::{LinkCheck, LinkOption};

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

/// The level with `link` added as its last rule.
fn with_rule(lv: &Config, link: &Link, action: Action, scope: Scope) -> Result<Config> {
    let sel = |s: &str| Sel::try_from(s.to_string()).map_err(anyhow::Error::msg);
    let mut with = lv.clone();
    with.rules.push(DepRule {
        from: sel(link.from)?,
        to: sel(link.to)?,
        via: None,
        kinds: link.kinds.to_vec(),
        scope,
        action,
        reason: None,
        loc: String::new(),
    });
    Ok(with)
}

/// Every edge a new rule covers, decided without and with it.
#[derive(Default)]
struct Tally {
    total: usize,
    changed: usize,
    /// Every covered pair goes from an independent brick to a normal one.
    never: bool,
    /// Rule locations deciding the covered edges now (`""`: the default).
    deciders: std::collections::BTreeSet<String>,
}

impl Tally {
    fn pair(
        &mut self,
        lv: &Config,
        with: &Config,
        (f, t): (&Brick, &Brick),
        kinds: &[&str],
        scopes: &[&str],
    ) {
        self.never &= f.independent && !t.independent;
        for (k, s) in kinds
            .iter()
            .flat_map(|k| scopes.iter().map(move |s| (k, s)))
        {
            let before = decide(lv, f, t, None, k, s);
            self.total += 1;
            if decide(with, f, t, None, k, s).action != before.action {
                self.changed += 1;
            }
            let by = before.rule.map_or_else(String::new, |(_, r)| r.loc.clone());
            self.deciders.insert(by);
        }
    }
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

impl Design {
    /// For the brick at `path`, every link it may draw to a sibling: each edge kind and scope
    /// with what the level's rules decide now.
    pub fn link_options(&self, path: &[String]) -> Result<Vec<LinkOption>> {
        let cfg = self.config()?;
        let (_, parent) = path.split_last().context("empty brick path")?;
        let level = cfg.node(parent).context("no such level")?;
        let from = cfg
            .brick_at(path)
            .with_context(|| format!("no brick {path:?}"))?;
        let mut out = Vec::new();
        for to in level.bricks.iter().filter(|b| b.name != from.name) {
            for kind in KINDS {
                for scope in ["prod", "test"] {
                    let d = decide(level, from, to, None, kind, scope);
                    out.push(LinkOption {
                        to: to.name.clone(),
                        kind: kind.into(),
                        scope: scope.into(),
                        action: action(d.action).into(),
                        rule: d.rule.map(|(_, r)| r.loc.clone()),
                    });
                }
            }
        }
        Ok(out)
    }

    /// A rule location (`/abs/dir/brickie.toml:12`) relative to the root config's directory,
    /// as people read it (`crates/core/brickie.toml:12`); other text is kept.
    fn short_loc(&self, loc: &str) -> String {
        let (path, line) = match loc.rsplit_once(':') {
            Some((p, l)) if l.chars().all(|c| c.is_ascii_digit()) => (p, Some(l)),
            _ => (loc, None),
        };
        let Some(rel) = crate::paths::to_rel(&self.root, std::path::Path::new(path)) else {
            return loc.to_string();
        };
        match line {
            Some(l) => format!("{rel}:{l}"),
            None => rel,
        }
    }

    /// Where a decision comes from, as a note names it.
    fn decider_name(&self, loc: Option<&str>) -> String {
        match loc {
            None => "several rules".into(),
            Some("") => "the default (no rule)".into(),
            Some(l) => self.short_loc(l),
        }
    }

    /// Whether adding `link` at `level` (brick names from the root; `[]`: the root) would
    /// change what the level decides: every edge it covers (each matching pair, kind and scope)
    /// is decided with and without it.
    pub fn link_check(&self, level: &[String], link: &Link) -> Result<LinkCheck> {
        let act = match link.action {
            "allow" => Action::Allow,
            "deny" => Action::Deny,
            a => bail!("action must be allow or deny, not '{a}'"),
        };
        let (scope, scopes): (Scope, &[&str]) = match link.scope {
            "prod" => (Scope::Prod, &["prod"]),
            "test" => (Scope::Test, &["test"]),
            "all" => (Scope::All, &["prod", "test"]),
            s => bail!("scope must be prod, test or all, not '{s}'"),
        };
        let cfg = self.config()?;
        let lv = cfg
            .node(level)
            .with_context(|| format!("no level {level:?}"))?;
        if let Some(r) = lv.rules.iter().find(|r| same_rule(r, link)) {
            return Ok(LinkCheck {
                redundant: true,
                why: format!("the same rule exists at {}", self.short_loc(&r.loc)),
                rule: Some(r.loc.clone()),
            });
        }
        let (froms, tos) = (matching(lv, link.from)?, matching(lv, link.to)?);
        let with = with_rule(lv, link, act, scope)?;
        let kinds: Vec<&str> = match link.kinds.is_empty() {
            true => KINDS.to_vec(),
            false => link.kinds.iter().map(String::as_str).collect(),
        };
        let mut t = Tally {
            never: true,
            ..Tally::default()
        };
        for f in &froms {
            for to in tos.iter().filter(|t| t.name != f.name) {
                t.pair(lv, &with, (f, to), &kinds, scopes);
            }
        }
        let Tally {
            total,
            changed,
            never,
            mut deciders,
        } = t;
        if changed > 0 {
            let why = format!("would {} {changed} of {total} edges", link.action);
            return Ok(LinkCheck {
                redundant: false,
                why,
                rule: None,
            });
        }
        if never && act == Action::Allow {
            return Ok(LinkCheck {
                redundant: true,
                why: "an allow from an independent brick to a normal brick never applies".into(),
                rule: None,
            });
        }
        let one = (deciders.len() == 1)
            .then(|| deciders.pop_first())
            .flatten();
        Ok(LinkCheck {
            redundant: true,
            why: format!(
                "already {} by {}",
                past(link.action),
                self.decider_name(one.as_deref())
            ),
            rule: one.filter(|d| !d.is_empty()),
        })
    }
}
