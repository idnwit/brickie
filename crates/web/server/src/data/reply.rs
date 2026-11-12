//! The answer of a change write ([`ChangeReply`]): the change after it and the preview levels
//! it touched, marked as `/api/graph?change=1` marks them, so the editor patches in place.

use super::{change_view, graph, mark};
use anyhow::Result;
use brickie_change::{Change, Op};
use brickie_web_api::{ChangeReply, ChangeView, LevelView, under};
use std::collections::BTreeSet;

/// The config files an op edits (a move also edits the level it moves into).
pub fn op_files(op: &Op) -> Vec<String> {
    let mut out = vec![op.file().to_string()];
    if let Op::MoveBrick { into_file, .. } = op {
        out.push(into_file.clone());
    }
    out
}

/// The files of the logged op `seq` (none when it is not in the log).
pub fn seq_files(ch: &Change, seq: u64) -> Result<Vec<String>> {
    let log = ch.log()?;
    Ok(log
        .ops
        .iter()
        .find(|o| o.seq == seq)
        .map_or_else(Vec::new, |o| op_files(&o.op)))
}

/// Every preview level, marked (empty when the preview or the cache does not load).
fn preview_levels(ch: &Change, view: &ChangeView) -> Vec<LevelView> {
    if view.error.is_some() {
        return Vec::new();
    }
    let Ok(l) = ch.preview(true).and_then(|p| p.check()) else {
        return Vec::new();
    };
    let mut levels = graph::levels(&l);
    mark(&mut levels, &l, view);
    levels
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}

/// The preview levels at `paths` (all when empty).
pub fn levels(ch: &Change, paths: &[String]) -> Result<Vec<LevelView>> {
    let view = change_view(ch)?;
    let mut out = preview_levels(ch, &view);
    if !paths.is_empty() {
        out.retain(|lv| paths.contains(&lv.path));
    }
    Ok(out)
}

/// The reply to a write that edited `files`: the levels of those files, their ancestors and
/// their direct children.
pub fn reply<T>(ch: &Change, done: T, files: &[String]) -> Result<ChangeReply<T>> {
    let change = change_view(ch)?;
    let hit: BTreeSet<&str> = (change.files.iter())
        .filter(|(_, f)| files.contains(f))
        .map(|(lv, _)| lv.as_str())
        .collect();
    let mut levels = preview_levels(ch, &change);
    levels.retain(|lv| {
        hit.iter()
            .any(|h| under(h, &lv.path) || parent(&lv.path) == *h)
    });
    Ok(ChangeReply {
        done,
        change,
        levels,
    })
}
