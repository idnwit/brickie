//! Notes on a write that succeeds: facts the writer would want to know about its ops.

use super::Design;
use super::links::Link;
use crate::paths::to_rel;
use brickie_check::config::Config;
use brickie_model::codes::DESIGN_LINK_DENIED;
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
    /// The note for one `add_link` op that no rule allows (it may be drafted, but blocks
    /// publishing), judged against `cfg`: the design just before the op, so earlier ops of the
    /// same write count. An op that cannot be judged gets no note.
    pub(super) fn link_note(&self, cfg: &Config, op: &Op) -> Option<String> {
        let Op::AddLink { file, link } = op else {
            return None;
        };
        let level = level_of(&self.root, cfg, file)?;
        let want = Link {
            from: &link.from,
            to: &link.to,
            kinds: &link.kinds,
            scope: link.scope.as_deref().unwrap_or("all"),
        };
        let check = self.link_check_in(cfg, &level, &want).ok()?;
        (!check.allowed).then(|| {
            format!(
                "{DESIGN_LINK_DENIED}: link {} -> {} is not allowed: {}; it blocks publishing until a rule allows it",
                link.from, link.to, check.why
            )
        })
    }
}
