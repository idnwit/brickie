//! The links a brick may draw to its siblings, and what a new link would be. Both ask the
//! shared decisions of `check::config` (`rule_allows`, `judge_link`).

use super::Design;
use anyhow::{Context, Result, bail};
use brickie_check::config::{
    Config, KINDS, Link as CfgLink, Scope, free, judge_link, kinds_of, rule_allows,
};
use brickie_model::design::{LinkCheck, LinkOption};

/// The link as asked for: `from -> to` (a sibling or `composite/child`), `kinds` (empty: all)
/// and `scope`.
pub struct Link<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub kinds: &'a [String],
    /// `prod`, `test` or `all`.
    pub scope: &'a str,
}

fn scope(s: &str) -> Result<Scope> {
    Ok(match s {
        "prod" => Scope::Prod,
        "test" => Scope::Test,
        "all" => Scope::All,
        s => bail!("scope must be prod, test or all, not '{s}'"),
    })
}

fn same(l: &CfgLink, want: &CfgLink) -> bool {
    let mut a = l.kinds.clone();
    let mut b = want.kinds.clone();
    a.sort();
    b.sort();
    l.from == want.from && l.to == want.to && a == b && l.scope == want.scope
}

impl Design {
    /// For the brick at `path`, every link it may draw to a sibling: each edge kind and scope
    /// with whether a rule of the level allows it.
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
                    let r = rule_allows(level, from, to, kind, scope);
                    out.push(LinkOption {
                        to: to.name.clone(),
                        kind: kind.into(),
                        scope: scope.into(),
                        allowed: r.is_some() || free(from, to),
                        rule: r.map(|r| r.loc.clone()),
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

    /// What adding `link` at `level` (brick names from the root; `[]`: the root) would be: a
    /// duplicate, allowed, or denied by the level's rules.
    pub fn link_check(&self, level: &[String], link: &Link) -> Result<LinkCheck> {
        self.link_check_in(&self.config()?, level, link)
    }

    /// [`Design::link_check`] against `cfg` instead of the working design.
    pub(super) fn link_check_in(
        &self,
        cfg: &Config,
        level: &[String],
        link: &Link,
    ) -> Result<LinkCheck> {
        let lv = cfg
            .node(level)
            .with_context(|| format!("no level {level:?}"))?;
        let want = CfgLink {
            from: link.from.to_string(),
            to: link.to.to_string(),
            kinds: link.kinds.to_vec(),
            scope: scope(link.scope)?,
            reason: String::new(),
            loc: String::new(),
        };
        let (to, _) = want.target();
        let (Some(f), Some(t)) = (lv.brick(link.from), lv.brick(to)) else {
            bail!(
                "'{}' or '{to}' is not a brick of level {}",
                link.from,
                lv.level()
            );
        };
        if let Some(l) = lv.links.iter().find(|l| same(l, &want)) {
            return Ok(LinkCheck {
                redundant: true,
                allowed: judge_link(lv, l).is_none(),
                why: format!("the same link exists at {}", self.short_loc(&l.loc)),
                rule: Some(l.loc.clone()),
            });
        }
        if let Some(why) = judge_link(lv, &want) {
            return Ok(LinkCheck {
                redundant: false,
                allowed: false,
                why,
                rule: None,
            });
        }
        let rule = (kinds_of(link.kinds).first())
            .and_then(|k| {
                rule_allows(
                    lv,
                    f,
                    t,
                    k,
                    if link.scope == "test" { "test" } else { "prod" },
                )
            })
            .map(|r| r.loc.clone());
        let why = match (&rule, free(f, t)) {
            (_, true) => format!("`{}` is independent: no rule is needed", t.name),
            (Some(r), _) => format!("allowed by {}", self.short_loc(r)),
            (None, _) => "allowed".into(),
        };
        Ok(LinkCheck {
            redundant: false,
            allowed: true,
            why,
            rule,
        })
    }
}
