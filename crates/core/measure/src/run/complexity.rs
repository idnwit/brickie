//! Function complexity from brickie-metrics, converted into the shared model.

pub use brickie_metrics::{Analysis, analyze};
use brickie_model::{FnTree, TreeNode};

/// The decision tree of the function starting at 1-based `line` of `src`.
pub fn fn_tree(src: &str, line: u32) -> Option<FnTree> {
    let t = brickie_metrics::fn_tree(src, line)?;
    Some(FnTree {
        label: t.label,
        line: t.line,
        end_line: t.end_line,
        lines: t.lines,
        paths: t.paths,
        cognitive: t.cognitive,
        cyclomatic: t.cyclomatic,
        max_depth: t.max_depth,
        nodes: t.nodes.into_iter().map(node).collect(),
    })
}

fn node(n: brickie_metrics::TreeNode) -> TreeNode {
    TreeNode {
        kind: n.kind,
        label: n.label,
        line: n.line,
        end_line: n.end_line,
        paths: n.paths,
        depth: n.depth,
    }
}
