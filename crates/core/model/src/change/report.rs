//! What a change reports: status, the semantic diff, tasks and the publish plan.

use super::{By, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub intent: String,
    /// The last `seq` in the log (0 when empty).
    pub head: u64,
    pub proposed: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub conflicts: usize,
    /// Seconds since the last `brickie measure` (`None`: no cache).
    pub cache_age_secs: Option<u64>,
}

/// One op as listed under a diff element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffOp {
    pub seq: u64,
    pub by: By,
    pub op: String,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<String>,
}

/// One element of the config tree the change touches, its ops coalesced: `before` is the base
/// value, `after` the value once the accepted and proposed ops are applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    pub file: String,
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub ops: Vec<DiffOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    /// A brick of the design with no measured module yet: create its code.
    Planned,
    /// A finding of `check --change` (drift between design and code).
    Drift,
    /// The preview does not load (a config error).
    Config,
    Proposed,
    Rejected,
    Conflict,
    /// A rule the change adds that no measured edge backs yet: write the code that uses it.
    Link,
    /// A rule the change adds without a `reason` (a warning, not a blocker).
    Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub kind: TaskKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// Finding id for drift tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanAction {
    /// Write a config file of the change.
    Write,
    /// Create a new file (scaffolding or a new config file).
    Create,
    /// Edit an existing file (a `mod` line, a workspace member).
    Edit,
    /// Create a directory.
    Mkdir,
    /// Scaffolding that is not done (the reason is in `note`).
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanFile {
    /// Relative to the root config's directory.
    pub path: String,
    pub action: PlanAction,
    pub note: String,
    /// The full content written (`None` for `mkdir` and `skip`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// What publish writes, in order; `blockers` lists why it would refuse (empty: it can run).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub files: Vec<PlanFile>,
    pub blockers: Vec<String>,
    /// Worth a look but not blocking (rules added without a reason).
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// A link the editor may draw from a brick to a sibling (same level): one edge kind and scope,
/// and what the rules of that level decide for it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkOption {
    pub to: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    /// `prod` or `test`.
    pub scope: String,
    /// `allow` or `deny`.
    pub action: String,
    /// `file:line` of the deciding rule; `None` when `deps.default` decides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// A canvas position drawn in the editor (UI only, never an op).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutHint {
    pub x: f64,
    pub y: f64,
}
