//! Helpers for the editor: building `move_brick` from brick paths and canvas layout hints (UI
//! only, never ops). The links a brick may draw are in `links`.

use super::Change;
use crate::paths::to_rel;
use anyhow::{Context, Result};
use brickie_check::config::Config;
use brickie_model::change::{LayoutHint, Op};
use std::collections::BTreeMap;

impl Change {
    pub(super) fn design(&self) -> Result<Config> {
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

    /// Canvas positions by brick path (`root/web/api`).
    pub fn layout(&self) -> Result<BTreeMap<String, LayoutHint>> {
        self.store.layout()
    }

    /// Set (`Some`) or clear the canvas position of a brick.
    pub fn set_layout(&self, brick: &str, hint: Option<LayoutHint>) -> Result<()> {
        self.store.set_layout(brick, hint)
    }
}
