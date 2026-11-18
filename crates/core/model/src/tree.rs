//! One function's decision tree: measured by the engine, drawn by the viewer.

use serde::{Deserialize, Serialize};

/// One node of a function's decision tree (`cc.py` `_tree`), nodes in pre-order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    /// `if` `else` `match` `arm` `loop` `closure` `cond_op` `qmark`.
    pub kind: String,
    /// What the node is about (a condition, a pattern, a loop head).
    pub label: String,
    /// First line of the node, 1-based.
    pub line: u32,
    /// Last line of the node, 1-based.
    pub end_line: u32,
    /// NPath of the subtree.
    pub paths: u64,
    /// Nesting depth of the node (0 at the function body).
    pub depth: u32,
}

/// The decision tree of one function plus its complexity numbers, as stored in the measure
/// cache. `label` is the function's name; `max_depth` is the deepest nesting; `paths` is the
/// NPath of the whole function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnTree {
    pub label: String,
    pub line: u32,
    pub end_line: u32,
    pub lines: u32,
    pub paths: u64,
    pub cognitive: u32,
    pub cyclomatic: u32,
    pub max_depth: u32,
    pub nodes: Vec<TreeNode>,
}
