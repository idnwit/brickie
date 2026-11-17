//! The design's own todo list: config self-consistency, read from the design files alone.
//! What the code lacks is `design check`.

use brickie_check::config::Config;
use brickie_model::codes::DESIGN_NO_REASON;
use brickie_model::design::{Task, TaskKind};

fn task(kind: TaskKind, text: String) -> Task {
    Task { kind, text }
}

/// The todo list of a design held as files: rules without a reason.
pub fn design_todo(cfg: &Config) -> Vec<Task> {
    let mut out = Vec::new();
    reasonless(cfg, &mut out);
    out
}

/// One task per rule without a reason (a preset's rule once).
fn reasonless(level: &Config, out: &mut Vec<Task>) {
    for r in &level.rules {
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
