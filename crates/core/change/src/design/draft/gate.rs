//! The publish gate: every link of the draft is allowed by a rule (`design/link-denied`, read
//! from the design alone), and the code matches the draft's structure: strict resolve over a
//! fresh cache, no partition error, no planned brick, no stale cache. Dependency findings never
//! block; they are reported as information, with the links no measured edge uses yet and the
//! links on a cycle.

use super::super::Design;
use super::super::tree::Files;
use crate::paths::to_rel;
use crate::preview::{cache_mismatch, planned};
use crate::report::design_todo;
use anyhow::Result;
use brickie_check::check::{self, Loaded};
use brickie_check::config::{Config, Partition, level_name};
use brickie_model::codes::{DESIGN_INVALID, DESIGN_LINK_DENIED, DESIGN_PLANNED, STALE_MEASURE};
use brickie_model::design::{Blocker, Gate, TaskKind};

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

/// The design-only part of the gate: denied links block, links on a cycle are information.
fn links(cfg: &Config, g: &mut Gate) {
    for t in design_todo(cfg) {
        match t.kind {
            TaskKind::LinkDenied => g.blocking.push(item(DESIGN_LINK_DENIED, t.text)),
            TaskKind::LinkCycle => g.info.push(t.text),
        }
    }
}

impl Design {
    /// The gate of the draft `files` against the measured cache on disk.
    pub(crate) fn gate(&self, files: &Files) -> Result<Gate> {
        let mut g = Gate::default();
        if let Ok(cfg) = self.preview(files)?.load_config() {
            links(&cfg, &mut g);
        }
        if !self.root.join(".brickie/cache.json").is_file() {
            let msg = "no measurements yet: run `brickie measure`".to_string();
            g.blocking.push(item(STALE_MEASURE, msg));
            return Ok(g);
        }
        let preview = self.preview(files)?;
        match check::load_from(&preview, &self.config_path()) {
            Ok(l) => self.loaded(&l, &mut g),
            Err(e) => match e.downcast_ref::<Partition>() {
                Some(p) => g.blocking.extend(p.0.iter().map(|i| Blocker {
                    file: Some(self.rel(&i.file)),
                    brick: Some(level_name(&i.level)),
                    member: i.member.clone(),
                    module: i.module.clone(),
                    ..item(i.code, i.message.clone())
                })),
                None => {
                    let msg = format!("{e:#}");
                    let code = match msg.contains("brickie measure") {
                        true => STALE_MEASURE,
                        false => DESIGN_INVALID,
                    };
                    g.blocking.push(item(code, msg));
                }
            },
        }
        Ok(g)
    }

    fn loaded(&self, l: &Loaded, g: &mut Gate) {
        if let Some(m) = cache_mismatch(&self.root, l) {
            g.blocking.push(item(STALE_MEASURE, m));
            return;
        }
        for d in &l.diagnostics {
            match d.code.starts_with("stale/") {
                true => g.blocking.push(item(&d.code, d.message.clone())),
                false => g.info.push(format!("{}: {}", d.code, d.message)),
            }
        }
        for s in check::planned_links(l) {
            g.info.push(format!(
                "planned link: {} -> {} in {} has no code edge yet ({})",
                s.link.from,
                s.link.to,
                level_name(&s.level),
                s.match_text(),
            ));
        }
        for path in planned(&l.cfg, &l.resolved) {
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
