//! The design's own todo list, read from the design files alone: links no rule allows (they
//! block publishing) and links on a cycle (information). What the code lacks is `brickie check`'s todo.

use brickie_check::check::{denied_text, design_links};
use brickie_check::config::{Config, level_name};
use brickie_model::design::{Task, TaskKind};

/// The todo list of a design held as files.
pub fn design_todo(cfg: &Config) -> Vec<Task> {
    let mut out = Vec::new();
    for s in design_links(cfg) {
        if let Some(why) = &s.denied {
            out.push(Task {
                kind: TaskKind::LinkDenied,
                text: denied_text(&s, why),
            });
        }
        if s.cycle {
            out.push(Task {
                kind: TaskKind::LinkCycle,
                text: format!(
                    "link {} -> {} in {} is on a cycle of links (information only)",
                    s.link.from,
                    s.link.to,
                    level_name(&s.level)
                ),
            });
        }
    }
    out
}
