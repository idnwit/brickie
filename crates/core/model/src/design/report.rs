//! What `design show` reports: the tree, its rules and limits, and the todo list.

use super::By;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    /// A brick with no measured module yet: create its code.
    Planned,
    /// The design does not check (an unfinished partition, a cache problem).
    Config,
    /// An allow rule between two named bricks that no measured edge uses yet.
    Link,
    /// A rule without a `reason`.
    Reason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub kind: TaskKind,
    pub text: String,
}

/// The design at one version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignView {
    pub v: u64,
    pub hash: String,
    pub by: By,
    pub at: u64,
    pub root: LevelView,
    pub todo: Vec<Task>,
}

/// One level of the tree: a config file and what it includes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelView {
    /// `root`, `root/web`.
    pub path: String,
    /// The level's config file, relative to the root config's directory.
    pub file: String,
    pub bricks: Vec<BrickView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<String>,
    /// `allow` or `deny`.
    pub default: String,
    pub cycles: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<LimitView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrickView {
    pub name: String,
    /// `crate`, `module` or `group`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    /// No measured module maps to the brick yet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub todo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Box<LevelView>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleView {
    /// The file the rule is written in (a level's own file or an included preset).
    pub file: String,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub via: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    pub scope: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// An allow rule between named bricks that no measured edge uses yet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub todo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitView {
    pub file: String,
    pub bricks: String,
    /// Metric -> limit.
    pub metrics: BTreeMap<String, u64>,
    pub level: String,
    pub scope: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanAction {
    /// Create a new file.
    Create,
    /// Edit an existing file (a `mod` line, a workspace member).
    Edit,
    /// Create a directory.
    Mkdir,
    /// Scaffolding that is not done (the reason is in `note`).
    Skip,
}

/// A code file scaffolded for a new brick.
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
