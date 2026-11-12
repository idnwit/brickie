//! The design editor's data: the open change as the viewer shows it and the request bodies of
//! the change endpoints. The op and record types are `brickie_model::change`, re-exported as
//! [`change`](crate::change).

pub use brickie_model::change;

use crate::LevelView;
use change::{DiffEntry, FileSheet, LayoutHint, Op, OpRecord, PlannedEdge, Status, Task, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The open change: status, the op log (with review state and conflicts), the semantic diff,
/// the open tasks and the canvas positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeView {
    pub status: Status,
    pub log: Vec<OpRecord>,
    pub diff: Vec<DiffEntry>,
    pub todo: Vec<Task>,
    /// Canvas positions by brick path (`root/web/api`); UI only, never ops.
    pub layout: BTreeMap<String, LayoutHint>,
    /// Level path (`root`, `root/web`) -> its config file, relative to the root config's
    /// directory (the `file` of an op on that level).
    pub files: BTreeMap<String, String>,
    /// Level path -> what a rule selector there may name (for client-side validation).
    #[serde(default)]
    pub levels: BTreeMap<String, LevelInfo>,
    /// Rules the change adds that no measured edge backs yet (also on each [`LevelView`]).
    #[serde(default)]
    pub planned_edges: Vec<PlannedEdge>,
    /// Every config file of the preview (its own rules, complexity, deps, exports, includes),
    /// by the same relative path as `files` values: the editor's Rules / Level / Files tabs.
    #[serde(default)]
    pub sheets: BTreeMap<String, FileSheet>,
    /// Why the preview does not load (the page then shows the current design).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeOpenRequest {
    pub intent: String,
}

/// A human op (accepted when recorded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeOpRequest {
    pub op: Op,
    #[serde(default)]
    pub note: Option<String>,
}

/// Move the brick at `from` into the brick at `into` (`[]`: the root level). Paths are brick
/// names from the root, without `root`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeMoveRequest {
    pub from: Vec<String>,
    pub into: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Accept or reject the op `seq` (a human review).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeReviewRequest {
    pub seq: u64,
    pub verdict: Verdict,
    #[serde(default)]
    pub note: Option<String>,
}

/// The links the brick at `path` (names from the root, without `root`) may draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeLinksRequest {
    pub path: Vec<String>,
}

/// Set (`Some`) or clear the canvas position of the brick at level path `brick`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeLayoutRequest {
    pub brick: String,
    pub hint: Option<LayoutHint>,
}

/// The names a level of the preview offers to rule selectors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelInfo {
    /// Child bricks, in config order.
    pub bricks: Vec<String>,
    /// Tags of the children, first-seen order.
    pub tags: Vec<String>,
    /// Every valid selector: `*`, each brick name, each `tag:<t>`.
    pub selectors: Vec<String>,
}

/// The answer of a change write: what it did (`done`), the change after it, and the preview
/// levels it touched (the level of each file it edited, their parents and direct children),
/// so the editor patches them in place. Levels that no longer exist are absent from
/// `change.files`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeReply<T> {
    pub done: T,
    pub change: ChangeView,
    pub levels: Vec<LevelView>,
}

/// Take back the human op `seq` (`None`: the latest accepted human op).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRetractRequest {
    #[serde(default)]
    pub seq: Option<u64>,
}

/// Re-record the conflicting op `seq` as a human op against the current design.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeReapplyRequest {
    pub seq: u64,
}

/// Would rule `from -> to` at `level` (brick names from the root, `[]`: the root) change
/// anything? `from` / `to` are selectors; `kinds` empty means all; `scope` is `prod`, `test`
/// or `all`; `action` `allow` or `deny`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeLinkCheckRequest {
    #[serde(default)]
    pub level: Vec<String>,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub kinds: Vec<String>,
    pub scope: String,
    pub action: String,
}

/// Preview levels by path (`root`, `root/web`); empty: every level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeLevelsRequest {
    #[serde(default)]
    pub paths: Vec<String>,
}
