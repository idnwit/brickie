//! The design editor's data: the open change as the viewer shows it and the request bodies of
//! the change endpoints. The op and record types are `brickie_model::change`, re-exported as
//! [`change`](crate::change).

pub use brickie_model::change;

use change::{DiffEntry, LayoutHint, Op, OpRecord, Status, Task, Verdict};
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
