//! Notes on a write that succeeds: facts the writer would want to know about its ops.

use super::Design;
use super::links::Link;
use crate::paths::to_rel;
use anyhow::Result;
use brickie_check::config::Config;
use brickie_model::design::Op;

/// The names of the bricks from the root to the level held by `file`.
fn level_of(root: &std::path::Path, cfg: &Config, file: &str) -> Option<Vec<String>> {
    let here = std::path::absolute(&cfg.file).ok()?;
    if to_rel(root, &here).as_deref() == Some(file) {
        return Some(cfg.path.clone());
    }
    (cfg.bricks.iter())
        .filter_map(|b| b.sub.as_deref())
        .find_map(|sub| level_of(root, sub, file))
}

impl Design {
    /// One note per `add_rule` op whose rule changes nothing at its level, judged against the
    /// design as it is now. An op that cannot be judged (the level or a brick it names is
    /// created by the same write) gets no note.
    pub fn rule_notes(&self, ops: &[Op]) -> Vec<String> {
        let Ok(cfg) = self.config() else {
            return Vec::new();
        };
        ops.iter()
            .filter_map(|op| self.rule_note(&cfg, op).ok().flatten())
            .collect()
    }

    fn rule_note(&self, cfg: &Config, op: &Op) -> Result<Option<String>> {
        let Op::AddRule { file, rule } = op else {
            return Ok(None);
        };
        let Some(level) = level_of(&self.root, cfg, file) else {
            return Ok(None);
        };
        let scope = rule.scope.as_deref().unwrap_or("prod");
        let link = Link {
            from: &rule.from,
            to: &rule.to,
            kinds: &rule.kinds,
            scope,
            action: &rule.action,
        };
        let check = self.link_check(&level, &link)?;
        Ok(check.redundant.then(|| {
            format!(
                "rule {} -> {} {} changes nothing: {}",
                rule.from, rule.to, rule.action, check.why
            )
        }))
    }
}
