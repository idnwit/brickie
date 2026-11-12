//! The open change as the editor shows it ([`ChangeView`]) and the change marks of the preview
//! levels: planned bricks (no measured module yet) and bricks an agent op proposes.

use anyhow::Result;
use brickie_change::{By, Change, Op, State};
use brickie_check::check::Loaded;
use brickie_check::config::{Config, level_name};
use brickie_web_api::{ChangeView, LevelView};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Status, log, diff, tasks and layout of the open change. `error` is set when the preview
/// does not load (`files` then lists no level).
pub fn change_view(ch: &Change) -> Result<ChangeView> {
    let (files, error) = match ch.preview(true).and_then(|p| p.load_config()) {
        Ok(cfg) => {
            let mut files = BTreeMap::new();
            level_files(&cfg, ch.root(), &mut files);
            (files, None)
        }
        Err(e) => (BTreeMap::new(), Some(format!("{e:#}"))),
    };
    Ok(ChangeView {
        status: ch.status()?,
        log: ch.log()?.ops,
        diff: ch.diff()?,
        todo: ch.todo().unwrap_or_default(),
        layout: ch.layout()?,
        files,
        error,
    })
}

fn level_files(cfg: &Config, root: &Path, out: &mut BTreeMap<String, String>) {
    let file = Path::new(&cfg.file);
    let rel = file.strip_prefix(root).unwrap_or(file);
    let rel = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    out.insert(level_name(&cfg.path), rel);
    for b in &cfg.bricks {
        if let Some(sub) = &b.sub {
            level_files(sub, root, out);
        }
    }
}

/// Level paths of the bricks added or renamed by agent ops still waiting for review.
fn proposed(view: &ChangeView) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let pending = view
        .log
        .iter()
        .filter(|r| r.by == By::Agent && r.state == State::Proposed && r.conflict.is_none());
    for r in pending {
        let (file, name) = match &r.op {
            Op::AddBrick { file, brick } => (file, &brick.name),
            Op::RenameBrick { file, to, .. } => (file, to),
            _ => continue,
        };
        for (level, f) in &view.files {
            if f == file {
                out.insert(format!("{level}/{name}"));
            }
        }
    }
    out
}

/// Mark planned and agent-proposed bricks on the preview levels.
pub fn mark(levels: &mut [LevelView], l: &Loaded, view: &ChangeView) {
    let planned: BTreeSet<String> = brickie_change::planned(&l.cfg, &l.resolved)
        .iter()
        .map(|p| level_name(p))
        .collect();
    let proposed = proposed(view);
    for b in levels.iter_mut().flat_map(|lv| lv.graph.bricks.iter_mut()) {
        if b.ghost {
            continue;
        }
        b.planned = planned.contains(&b.path);
        b.proposed = proposed.contains(&b.path);
    }
}
