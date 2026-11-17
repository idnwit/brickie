//! The publish gate is `check` on the draft: the same load, blocked by any error diagnostic,
//! any planned brick and a missing, mismatched or stale cache. Links the code does not use yet
//! are planned work and never block.

use super::super::Design;
use super::super::tree::Files;
use crate::paths::to_rel;
use crate::preview::cache_mismatch;
use anyhow::Result;
use brickie_check::check::{self, Loaded};
use brickie_check::config::level_name;
use brickie_model::codes::{DESIGN_INVALID, DESIGN_PLANNED, STALE_MEASURE};
use brickie_model::design::{Blocker, Gate};

fn item(code: &str, message: String) -> Blocker {
    Blocker {
        code: code.to_string(),
        message,
        file: None,
        brick: None,
        member: None,
        module: None,
    }
}

impl Design {
    /// The gate of the draft `files`: `check` over the measured cache on disk.
    pub(crate) fn gate(&self, files: &Files) -> Result<Gate> {
        let mut g = Gate::default();
        if !self.root.join(".brickie/cache.json").is_file() {
            let msg = "no measurements yet: run `brickie measure`".to_string();
            g.blocking.push(item(STALE_MEASURE, msg));
            return Ok(g);
        }
        let preview = self.preview(files)?;
        match check::load_from(&preview, &self.config_path()) {
            Ok(l) => self.loaded(&l, &mut g),
            Err(e) => {
                let msg = format!("{e:#}");
                let code = match msg.contains("brickie measure") {
                    true => STALE_MEASURE,
                    false => DESIGN_INVALID,
                };
                g.blocking.push(item(code, msg));
            }
        }
        Ok(g)
    }

    fn loaded(&self, l: &Loaded, g: &mut Gate) {
        if let Some(m) = cache_mismatch(&self.root, l) {
            g.blocking.push(item(STALE_MEASURE, m));
            return;
        }
        for i in &l.issues {
            g.blocking.push(Blocker {
                file: Some(self.rel(&i.file)),
                brick: Some(level_name(&i.level)),
                member: i.member.clone(),
                module: i.module.clone(),
                ..item(i.code, i.message.clone())
            });
        }
        for d in &l.diagnostics {
            // Config issues block through `l.issues` above.
            if !d.code.starts_with("config/") {
                g.blocking.push(item(&d.code, d.message.clone()));
            }
        }
        for path in l.todo.bricks.iter().cloned() {
            let Some(b) = l.cfg.brick_at(&path) else {
                continue;
            };
            let parent = &path[..path.len() - 1];
            let file = l.cfg.node(parent).map(|lv| self.rel(&lv.file));
            let want = match &b.path {
                Some(p) => format!("create the code of its children at {p}/"),
                None if b.members.is_empty() => "give it members".to_string(),
                None => format!("create the code for members {:?}", b.members),
            };
            let name = level_name(&path);
            g.blocking.push(Blocker {
                file,
                brick: Some(name.clone()),
                member: (!b.members.is_empty()).then(|| b.members.join(", ")),
                ..item(
                    DESIGN_PLANNED,
                    format!("{DESIGN_PLANNED}: brick {name} has no code yet; {want}"),
                )
            });
        }
    }

    /// A config file path as the loader names it, relative to the root.
    fn rel(&self, file: &str) -> String {
        let abs = std::path::absolute(file).unwrap_or_default();
        to_rel(&self.root, &abs).unwrap_or_else(|| file.to_string())
    }
}
