//! What `design show` reports: the tree, its rules and limits, and the config todo list.

use super::{By, Track};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// A link no rule allows (`design/link-denied`): it blocks publishing.
    LinkDenied,
    /// A link on a cycle of its level's links: information only.
    LinkCycle,
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
    /// The tree shown: the draft when one exists, else the published tree.
    pub track: Track,
    /// The draft's base: the published version it was forked from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<u64>,
    pub hash: String,
    pub by: By,
    pub at: u64,
    pub root: LevelView,
    pub todo: Vec<Task>,
}

/// Tidiness score of a level's automatic top-down layout (computed from its bricks and links).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutScore {
    /// Overall tidiness score in `0..=100` (100 = clean, no crossings, box hits or excess bends).
    pub score: u8,
    pub edges: usize,
    pub crossings: usize,
    pub box_hits: usize,
    pub bends: usize,
}

impl LayoutScore {
    /// Score from layout metrics: each box hit costs 20, each crossing 8, each bend past 2 per
    /// edge costs 1.
    pub fn from_metrics(edges: usize, crossings: usize, box_hits: usize, bends: usize) -> Self {
        let penalty = if edges == 0 {
            0
        } else {
            box_hits
                .saturating_mul(20)
                .saturating_add(crossings.saturating_mul(8))
                .saturating_add(bends.saturating_sub(edges.saturating_mul(2)))
        };
        Self {
            score: 100usize.saturating_sub(penalty) as u8,
            edges,
            crossings,
            box_hits,
            bends,
        }
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<LimitView>,
    /// Automatic top-down layout tidiness score of this level's bricks and links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutScore>,
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
    /// An independent lib (`independent = true`): usable only within its parent's subtree.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub independent: bool,
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
    pub kinds: Vec<String>,
    pub scope: String,
    pub reason: String,
}

/// A `[[link]]` of a level, judged against the level's rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkView {
    /// The file the link is written in.
    pub file: String,
    pub from: String,
    /// A sibling brick, or `composite/child`.
    pub to: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    pub scope: String,
    pub reason: String,
    /// Why no rule allows it (`design/link-denied`); `None` for a valid link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denied: Option<String>,
    /// The link is on a cycle of its level's links (information only).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cycle: bool,
}

/// A `[[complexity]]` rule of a level: the metric limits that apply to the bricks it selects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitView {
    pub file: String,
    pub bricks: String,
    /// Metric -> limit.
    pub metrics: BTreeMap<String, u64>,
    pub scope: String,
}

/// What scaffolding does to one path (see [`PlanFile`]).
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
