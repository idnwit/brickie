//! Per-function rows, the metric names and one function's decision tree.

use super::Params;
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnRow {
    pub qname: String,
    pub file: String,
    pub line: u32,
    pub brick: String,
    pub scope: String,
    pub cognitive: u32,
    pub cyclomatic: u32,
    pub nesting: u32,
    pub paths: u64,
    pub params: u32,
    pub lines: u32,
}

impl FnRow {
    pub fn metric(&self, name: &str) -> u64 {
        match name {
            "cyclomatic" => self.cyclomatic.into(),
            "nesting" => self.nesting.into(),
            "paths" => self.paths,
            "params" => self.params.into(),
            "lines" => self.lines.into(),
            _ => self.cognitive.into(),
        }
    }
}

pub const METRICS: [&str; 6] = [
    "cognitive",
    "cyclomatic",
    "nesting",
    "paths",
    "params",
    "lines",
];

/// Indices of the functions that pass the filters, sorted by the params (stable; ties by
/// name), before the row limit.
pub fn fn_rows(all: &[FnRow], p: &Params) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..all.len())
        .filter(|&i| {
            let f = &all[i];
            (p.brick.is_empty() || p.bricks().contains(&f.brick.as_str()))
                && (p.scope() == "all" || f.scope == p.scope())
        })
        .collect();
    let sort = p.sort();
    idx.sort_by(|&a, &b| {
        let (x, y) = (&all[a], &all[b]);
        let o = if sort == "name" {
            x.qname.cmp(&y.qname)
        } else {
            x.metric(sort).cmp(&y.metric(sort))
        };
        // Metrics sort descending by default, names ascending.
        let reverse = if sort == "name" {
            p.fn_dir == "desc"
        } else {
            p.desc()
        };
        let o = if reverse { o.reverse() } else { o };
        o.then_with(|| x.qname.cmp(&y.qname))
            .then(x.file.cmp(&y.file))
    });
    idx
}
