//! The versioned design as the editor shows it ([`DesignState`]), read from the design files.

mod conflict;
mod layout;

pub use conflict::first_conflict;
pub use layout::{clear_level, read_layout, set_hint};

use anyhow::Result;
use brickie_change::{Design, DesignView};
use brickie_web_api::design::LevelView as DesignLevel;
use brickie_web_api::{DesignState, HISTORY, LevelInfo};
use std::collections::BTreeMap;

/// The design at the head of its working track, with the draft's state. `error` says why the
/// tree does not show (`files` is then empty).
pub fn state(d: &Design) -> Result<DesignState> {
    let log = d.log()?;
    let head = d.head()?;
    let published = d.published_head()?.v;
    let forked_from = d.forked_from()?;
    let skip = log.len().saturating_sub(HISTORY);
    let mut out = DesignState {
        v: head.v,
        published,
        forked_from,
        hash: head.hash,
        by: head.by,
        history: log.into_iter().skip(skip).collect(),
        todo: Vec::new(),
        layout: read_layout(d.root()),
        files: BTreeMap::new(),
        levels: BTreeMap::new(),
        sheets: d.sheets()?,
        notice: None,
        error: None,
    };
    match d.show_design() {
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
