//! The design editor's data: the versioned design as the viewer edits it ([`DesignState`]),
//! the request bodies of the design endpoints and the reply of a write. The op and version
//! types are `brickie_model::design`, re-exported as [`design`](crate::design).

use crate::LevelView;
use crate::design::{By, FileSheet, Op, Task, Version};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The design at its head version: history, tasks, the per-file sheets and what the canvas
/// needs (layout hints, level files, selectors, planned edges).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignState {
    /// The head version: the `base` of the next write.
    pub v: u64,
    pub hash: String,
    pub by: By,
    /// The latest versions, oldest first (at most [`HISTORY`]).
    pub history: Vec<Version>,
    pub todo: Vec<Task>,
    /// Canvas positions by brick path (`root/web/api`); UI only, never versioned.
    pub layout: BTreeMap<String, LayoutHint>,
    /// Level path (`root`, `root/web`) -> its config file, relative to the root config's
    /// directory (the `file` of an op on that level).
    pub files: BTreeMap<String, String>,
    /// Level path -> what a rule selector there may name (for client-side validation).
    #[serde(default)]
    pub levels: BTreeMap<String, LevelInfo>,
    /// Allow rules between named bricks that no measured edge uses yet (also on each
    /// [`LevelView`]).
    #[serde(default)]
    pub planned_edges: Vec<PlannedEdge>,
    /// Every config file of the design (its own rules, complexity, deps, exports, includes),
    /// by the same relative path as `files` values.
    #[serde(default)]
    pub sheets: BTreeMap<String, FileSheet>,
    /// Why the design does not load or check (the page then shows what it can).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// How many versions [`DesignState::history`] carries.
pub const HISTORY: usize = 50;

/// A canvas position drawn in the editor (UI only, never an op).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayoutHint {
    pub x: f64,
    pub y: f64,
}

/// An allow rule between named bricks that no measured edge uses yet: the canvas draws it
/// dashed. Named by identity (`file`, `from`, `to`, `kinds`, `scope`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedEdge {
    /// Level path the rule applies to (`root`, `root/web`).
    pub level: String,
    /// Physical file of the rule.
    pub file: String,
    pub from: String,
    pub to: String,
    /// Empty: all kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    /// `prod`, `test` or `all`.
    pub scope: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The names a level offers to rule selectors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelInfo {
    /// Child bricks, in config order.
    pub bricks: Vec<String>,
    /// Tags of the children, first-seen order.
    pub tags: Vec<String>,
    /// Every valid selector: `*`, each brick name, each `tag:<t>`.
    pub selectors: Vec<String>,
}

/// Apply `ops` on top of version `base` as one human version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditRequest {
    pub base: u64,
    pub ops: Vec<Op>,
}

/// Move the brick at `from` into the brick at `into` (`[]`: the root level). Paths are brick
/// names from the root, without `root`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRequest {
    pub base: u64,
    pub from: Vec<String>,
    pub into: Vec<String>,
}

/// Revert the file changes of version `v` (`None`: the head) as a new human version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoRequest {
    pub base: u64,
    #[serde(default)]
    pub v: Option<u64>,
}

/// A write request: based on a version, which a retry after `design/stale` moves to the head.
pub trait Based {
    fn rebase(&mut self, v: u64);
}

impl Based for EditRequest {
    fn rebase(&mut self, v: u64) {
        self.base = v;
    }
}

impl Based for MoveRequest {
    fn rebase(&mut self, v: u64) {
        self.base = v;
    }
}

impl Based for UndoRequest {
    fn rebase(&mut self, v: u64) {
        self.base = v;
    }
}

/// The answer of a write: whether it made a version, the design after it and every level of
/// the design (the page patches them in place).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditReply {
    /// `false`: the ops left every file as it was; no version was added.
    pub changed: bool,
    pub design: DesignState,
    pub levels: Vec<LevelView>,
}

/// The links the brick at `path` (names from the root, without `root`) may draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinksRequest {
    pub path: Vec<String>,
}

/// Set (`Some`) or clear the canvas position of the brick at level path `brick`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutRequest {
    pub brick: String,
    pub hint: Option<LayoutHint>,
}

/// Clear the canvas positions of every brick of `level` (`root`, `root/web`): auto layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayoutRequest {
    pub level: String,
}

/// Would rule `from -> to` at `level` (brick names from the root, `[]`: the root) change
/// anything? `from` / `to` are selectors; `kinds` empty means all; `scope` is `prod`, `test`
/// or `all`; `action` `allow` or `deny`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCheckRequest {
    #[serde(default)]
    pub level: Vec<String>,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub kinds: Vec<String>,
    pub scope: String,
    pub action: String,
}

/// Levels of the design by path (`root`, `root/web`); empty: every level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelsRequest {
    #[serde(default)]
    pub paths: Vec<String>,
}
