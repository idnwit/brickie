//! Tasks for the coding agent (and the human): what the design asks that the code lacks,
//! computed against the live design and the measured cache.

use crate::preview::{cache_mismatch, planned};
use brickie_check::check::{Loaded, filter_text, link_states};
use brickie_check::config::{Config, Pending, level_name};
use brickie_model::codes::{
    CONFIG_UNASSIGNED, DESIGN_NO_REASON, DESIGN_PLANNED, DESIGN_PLANNED_EDGE,
};
use brickie_model::design::{Task, TaskKind};
use std::path::Path;

fn task(kind: TaskKind, text: String) -> Task {
    Task { kind, text }
}

/// Everything to do, plus the planned brick paths and unbacked rule locations for marking the
/// tree. `loaded` is the draft check of the design (`Err`: why it does not check).
pub struct Todo {
    pub tasks: Vec<Task>,
    pub planned: Vec<Vec<String>>,
    pub unbacked: Vec<String>,
}

pub fn todo(root: &Path, cfg: &Config, loaded: Result<&Loaded, String>) -> Todo {
    let mut t = Todo {
        tasks: Vec::new(),
        planned: Vec::new(),
        unbacked: Vec::new(),
    };
    match loaded {
        Err(e) => t.tasks.push(task(TaskKind::Config, e)),
        Ok(l) => match cache_mismatch(root, l) {
            Some(m) => t.tasks.push(task(TaskKind::Config, m)),
            None => measured(l, &mut t),
        },
    }
    reasonless(cfg, &mut t.tasks);
    t
}

fn measured(l: &Loaded, t: &mut Todo) {
    let pending = l.resolved.pending.iter();
    t.tasks
        .extend(pending.map(|p| task(TaskKind::Config, pending_text(p))));
    t.planned = planned(&l.cfg, &l.resolved);
    for p in &t.planned {
        let text = format!("{DESIGN_PLANNED}: {}", planned_text(&l.cfg, p));
        t.tasks.push(task(TaskKind::Planned, text));
    }
    for link in link_states(l) {
        let text = format!(
            "{DESIGN_PLANNED_EDGE}: rule {} -> {} in {} allows an edge no measured code uses yet (state: {}; {})",
            link.from,
            link.to,
            level_name(&link.level),
            link.state.name(),
            filter_text(&link),
        );
        t.unbacked.push(link.loc);
        t.tasks.push(task(TaskKind::Link, text));
    }
}

/// One task per rule without a reason (a preset's rule once).
fn reasonless(level: &Config, out: &mut Vec<Task>) {
    for r in &level.deps.rules {
        if r.reason.as_deref().is_some_and(|s| !s.trim().is_empty()) {
            continue;
        }
        let text = format!(
            "{DESIGN_NO_REASON}: rule {} -> {} in {} has no reason; say why with `brickie design rule set`",
            r.from,
            r.to,
            level.level()
        );
        if !out.iter().any(|t| t.text == text) {
            out.push(task(TaskKind::Reason, text));
        }
    }
    for b in &level.bricks {
        if let Some(sub) = &b.sub {
            reasonless(sub, out);
        }
    }
}

/// A tolerated partition error of the draft, with how to finish it.
pub fn pending_text(p: &Pending) -> String {
    let hint = match p.module {
        Some(_) if p.code == CONFIG_UNASSIGNED => {
            "; add it to the members of one of the composite's bricks"
        }
        _ => "",
    };
    format!("{}{hint}", p.message)
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
