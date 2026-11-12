//! Helpers for the editor: building `move_brick` from brick paths, the links a brick may draw
//! to its siblings, and canvas layout hints (UI only, never ops).

use super::Change;
use crate::paths::to_rel;
use anyhow::{Context, Result};
use brickie_check::config::{Action, Brick, Config, DepRule, KINDS, Sel};
use brickie_model::change::{LayoutHint, LinkOption, Op};
use std::collections::BTreeMap;

fn sel(s: &Sel, b: &Brick) -> Option<u32> {
    match s {
        Sel::Any => Some(1),
        Sel::Tag(t) => b.tags.contains(t).then_some(2),
        Sel::Brick(n) => (n == &b.name).then_some(3),
    }
}

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
        let (Some(a), Some(b)) = (sel(&r.from, from), sel(&r.to, to)) else {
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

fn action(a: Action) -> String {
    if a == Action::Allow { "allow" } else { "deny" }.into()
}

impl Change {
    fn design(&self) -> Result<Config> {
        self.preview(true)?.load_config()
    }

    fn level_file(&self, cfg: &Config, parent: &[String]) -> Result<String> {
        let level = cfg
            .node(parent)
            .with_context(|| format!("no level {parent:?}"))?;
        to_rel(&self.root, std::path::Path::new(&level.file))
            .context("level file outside the workspace")
    }

    /// `move_brick` for the brick at `from` (path from the root) into the brick at `to_parent`
    /// (`[]`: the root level), against the current design.
    pub fn move_op(&self, from: &[String], to_parent: &[String]) -> Result<Op> {
        let cfg = self.design()?;
        let (name, parent) = from.split_last().context("empty brick path")?;
        cfg.brick_at(from)
            .with_context(|| format!("no brick {from:?}"))?;
        let file = self.level_file(&cfg, parent)?;
        let (into_file, into) = match to_parent.split_last() {
            None => (self.config.clone(), None),
            Some((p, above)) => (self.level_file(&cfg, above)?, Some(p.clone())),
        };
        Ok(Op::MoveBrick {
            file,
            name: name.clone(),
            into_file,
            into,
            path: None,
        })
    }

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
                        action: action(r.map_or(level.deps.default, |r| r.action)),
                        rule: r.map(|r| r.loc.clone()),
                    });
                }
            }
        }
        Ok(out)
    }

    /// Canvas positions by brick path (`root/web/api`).
    pub fn layout(&self) -> Result<BTreeMap<String, LayoutHint>> {
        self.store.layout()
    }

    /// Set (`Some`) or clear the canvas position of a brick.
    pub fn set_layout(&self, brick: &str, hint: Option<LayoutHint>) -> Result<()> {
        self.store.set_layout(brick, hint)
    }
}
