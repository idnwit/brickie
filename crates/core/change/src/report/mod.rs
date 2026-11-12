//! What a change reports to its readers: status, the semantic diff and the todo list.

mod diff;
mod edges;
mod todo;

use diff::diff;
pub use todo::{pending_text, planned_text};

use crate::session::Change;
use anyhow::Result;
use brickie_model::change::{DiffEntry, State, Status, Task, TaskKind};

impl Change {
    pub fn status(&self) -> Result<Status> {
        let log = self.log()?;
        let count = |s: State| log.ops.iter().filter(|o| o.state == s).count();
        let cache = self.root.join(".brickie/cache.json");
        let cache_age_secs = std::fs::metadata(cache)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs());
        Ok(Status {
            intent: self.intent()?,
            head: log.entries.last().map_or(0, |e| e.seq()),
            proposed: count(State::Proposed),
            accepted: count(State::Accepted),
            rejected: count(State::Rejected),
            conflicts: log.ops.iter().filter(|o| o.conflict.is_some()).count(),
            cache_age_secs,
        })
    }

    /// The change by element, ops coalesced.
    pub fn diff(&self) -> Result<Vec<DiffEntry>> {
        Ok(diff(&self.log()?.ops))
    }

    /// Concrete tasks: planned bricks, drift findings of the preview, ops to look at.
    pub fn todo(&self) -> Result<Vec<Task>> {
        let log = self.log()?;
        let mut tasks = todo::from_ops(&log.ops);
        tasks.extend(todo::from_preview(
            &self.preview(true)?,
            &self.base_preview()?,
        ));
        let edges = self.planned_edges()?;
        let allowed = edges.iter().filter(|e| e.action == "allow");
        tasks.extend(allowed.map(|e| todo::edge_task(TaskKind::Link, edges::unbacked(e), e)));
        let reasonless = self.reasonless_rules()?;
        tasks.extend(
            (reasonless.iter()).map(|e| todo::edge_task(TaskKind::Reason, edges::no_reason(e), e)),
        );
        Ok(tasks)
    }
}
