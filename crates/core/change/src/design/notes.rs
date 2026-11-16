//! Notes on a write that succeeds: facts the writer would want to know about its ops.

use super::Design;
use super::links::Link;
use crate::paths::to_rel;
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
    /// The note for one `add_rule` op whose rule changes nothing at its level, judged against
    /// `cfg`: the design just before the op, so earlier ops of the same write count. An op that
    /// cannot be judged (the level or a brick it names does not exist yet) gets no note.
    pub(super) fn rule_note(&self, cfg: &Config, op: &Op) -> Option<String> {
        let Op::AddRule { file, rule } = op else {
            return None;
        };
        let level = level_of(&self.root, cfg, file)?;
        let scope = rule.scope.as_deref().unwrap_or("prod");
        let link = Link {
            from: &rule.from,
            to: &rule.to,
            kinds: &rule.kinds,
            scope,
            action: &rule.action,
        };
        let check = self.link_check_in(cfg, &level, &link).ok()?;
        check.redundant.then(|| {
            format!(
                "rule {} -> {} {} changes nothing: {}",
                rule.from, rule.to, rule.action, check.why
            )
        })
    }
}
