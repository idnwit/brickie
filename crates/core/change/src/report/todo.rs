//! Tasks for the coding agent (and the human): what the design asks that the code lacks.

use crate::preview::{Preview, planned};
use brickie_check::config::{Config, level_name};
use brickie_model::Level;
use brickie_model::change::{OpRecord, State, Task, TaskKind};
use brickie_model::codes::CHANGE_PLANNED;
use std::collections::BTreeSet;

fn task(kind: TaskKind, text: String) -> Task {
    Task {
        kind,
        text,
        seq: None,
        id: None,
        help: None,
    }
}

/// Ops waiting for review, rejected (with the reason) or in conflict.
pub fn from_ops(ops: &[OpRecord]) -> Vec<Task> {
    let mut out = Vec::new();
    for o in ops {
        let what = format!("op {} {} {} {}", o.seq, o.op.name(), o.file, o.key);
        let (kind, text) = if let Some(c) = &o.conflict {
            (TaskKind::Conflict, format!("{what} is in conflict: {c}"))
        } else {
            match o.state {
                State::Proposed => (TaskKind::Proposed, format!("{what} waits for review")),
                State::Rejected => {
                    let note = o.review.as_ref().and_then(|r| r.note.as_deref());
                    let why = note.unwrap_or("no note");
                    (TaskKind::Rejected, format!("{what} was rejected: {why}"))
                }
                State::Accepted => continue,
            }
        };
        out.push(Task {
            seq: Some(o.seq),
            ..task(kind, text)
        });
    }
    out
}

/// Planned bricks and the unaccepted findings of `check` over the preview that the change
/// introduces: findings of `base` (the design as opened, same cache) are not drift.
pub fn from_preview(preview: &Preview, base: &Preview) -> Vec<Task> {
    let loaded = match preview.check() {
        Ok(l) => l,
        Err(e) => return vec![task(TaskKind::Config, format!("{e:#}"))],
    };
    if let Some(m) = preview.cache_mismatch(&loaded) {
        return vec![task(TaskKind::Config, m)];
    }
    let known: BTreeSet<String> = base
        .check()
        .map(|b| b.diagnostics.into_iter().map(|d| d.id).collect())
        .unwrap_or_default();
    let mut out: Vec<Task> = planned(&loaded.cfg, &loaded.resolved)
        .iter()
        .map(|p| {
            let text = format!("{CHANGE_PLANNED}: {}", planned_text(&loaded.cfg, p));
            task(TaskKind::Planned, text)
        })
        .collect();
    for d in loaded
        .diagnostics
        .iter()
        .filter(|d| !d.accepted && !known.contains(&d.id))
    {
        let at = d
            .span
            .as_ref()
            .map(|s| format!(" at {}:{}", s.file, s.line))
            .unwrap_or_default();
        let level = if d.level == Level::Error {
            "error"
        } else {
            "warning"
        };
        out.push(Task {
            id: Some(d.id.clone()),
            help: d.help.clone(),
            ..task(
                TaskKind::Drift,
                format!("{level}[{}] {}{at}", d.code, d.message),
            )
        });
    }
    out
}

/// `brick root/x has no measured module yet; create the code for members [..]`.
pub fn planned_text(cfg: &Config, path: &[String]) -> String {
    let Some(b) = cfg.brick_at(path) else {
        return level_name(path);
    };
    let what = match &b.path {
        Some(p) => format!("composite at {p}/ (create its children's code)"),
        None if b.members.is_empty() => "no members: set its members".to_string(),
        None => format!("create the code for members {:?}", b.members),
    };
    format!(
        "brick {} has no measured module yet; {what}",
        level_name(path)
    )
}
