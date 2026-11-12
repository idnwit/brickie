//! One function's decision tree: measured by the engine, drawn by the viewer.

use serde::{Deserialize, Serialize};

/// One node of a function's decision tree (`cc.py` `_tree`), nodes in pre-order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    /// `if` `else` `match` `arm` `loop` `closure` `cond_op` `qmark`.
    pub kind: String,
    pub label: String,
    pub line: u32,
    pub end_line: u32,
    /// NPath of the subtree.
    pub paths: u64,
    pub depth: u32,
}

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
