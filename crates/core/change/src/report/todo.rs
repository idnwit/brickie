//! The design's own todo list, read from the design files alone: links no rule allows (they
//! block publishing) and links on a cycle (information). What the code lacks is `design check`.

use brickie_check::check::design_links;
use brickie_check::config::{Config, LinkState, level_name};
use brickie_model::codes::DESIGN_LINK_DENIED;
use brickie_model::design::{Task, TaskKind};

/// `a -> b` in level `root/web`.
fn named(s: &LinkState) -> String {
    format!(
        "link {} -> {} in {}",
        s.link.from,
        s.link.to,
        level_name(&s.level)
    )
}

/// The message of a denied link, as the todo list and the publish gate print it.
pub fn denied_text(s: &LinkState, why: &str) -> String {
    format!(
        "{DESIGN_LINK_DENIED}: {} is not allowed: {why}; add a [[rule]] that allows it \
         (`brickie design rule add`) or remove the link",
        named(s)
    )
}

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
                text: format!("{} is on a cycle of links (information only)", named(&s)),
            });
        }
    }
    out
}
