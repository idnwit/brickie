//! The versioned design as the editor shows it ([`DesignState`]) and the edit-mode marks of
//! the levels: planned bricks (no measured module yet) and planned edges (allow rules no
//! measured edge uses yet).

mod layout;

pub use layout::{clear_level, read_layout, set_hint};

use anyhow::Result;
use brickie_change::{Design, DesignView};
use brickie_check::check::Loaded;
use brickie_check::config::level_name;
use brickie_web_api::design::LevelView as DesignLevel;
use brickie_web_api::{DesignState, HISTORY, LevelInfo, LevelView, PlannedEdge};
use std::collections::{BTreeMap, BTreeSet};

/// The design at its head. `error` says why the tree does not show (`files` is then empty).
pub fn state(d: &Design) -> Result<DesignState> {
    let log = d.log()?;
    let head = log
        .last()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no version"))?;
    let skip = log.len().saturating_sub(HISTORY);
    let mut out = DesignState {
        v: head.v,
        hash: head.hash,
        by: head.by,
        history: log.into_iter().skip(skip).collect(),
        todo: Vec::new(),
        layout: read_layout(d.root()),
        files: BTreeMap::new(),
        levels: BTreeMap::new(),
        planned_edges: Vec::new(),
        sheets: d.sheets()?,
        error: None,
    };
    match d.show() {
        Ok(view) => fill(&mut out, &view),
        Err(e) => out.error = Some(format!("{e:#}")),
    }
    Ok(out)
}

fn fill(out: &mut DesignState, view: &DesignView) {
    out.todo = view.todo.clone();
    let mut stack = vec![&view.root];
    while let Some(lv) = stack.pop() {
        out.files.insert(lv.path.clone(), lv.file.clone());
        out.levels.insert(lv.path.clone(), level_info(lv));
        out.planned_edges
            .extend(lv.rules.iter().filter(|r| r.todo).map(|r| PlannedEdge {
                level: lv.path.clone(),
                file: r.file.clone(),
                from: r.from.clone(),
                to: r.to.clone(),
                kinds: r.kinds.clone(),
                scope: r.scope.clone(),
                action: r.action.clone(),
                reason: r.reason.clone(),
            }));
        stack.extend(lv.bricks.iter().filter_map(|b| b.children.as_deref()));
    }
}

/// The bricks, tags and valid rule selectors of a level.
fn level_info(lv: &DesignLevel) -> LevelInfo {
    let bricks: Vec<String> = lv.bricks.iter().map(|b| b.name.clone()).collect();
    let mut tags: Vec<String> = Vec::new();
    for t in lv.bricks.iter().flat_map(|b| &b.tags) {
        if !tags.contains(t) {
            tags.push(t.clone());
        }
    }
    let mut selectors = vec!["*".to_string()];
    selectors.extend(bricks.iter().cloned());
    selectors.extend(tags.iter().map(|t| format!("tag:{t}")));
    LevelInfo {
        bricks,
        tags,
        selectors,
    }
}

/// Mark planned bricks and planned edges on the levels.
pub fn mark(levels: &mut [LevelView], l: &Loaded, st: &DesignState) {
    let planned: BTreeSet<String> = brickie_change::planned(&l.cfg, &l.resolved)
        .iter()
        .map(|p| level_name(p))
        .collect();
    for lv in levels.iter_mut() {
        lv.planned_edges = (st.planned_edges.iter())
            .filter(|e| e.level == lv.path)
            .cloned()
            .collect();
    }
    for b in levels.iter_mut().flat_map(|lv| lv.graph.bricks.iter_mut()) {
        b.planned = !b.ghost && planned.contains(&b.path);
    }
}
