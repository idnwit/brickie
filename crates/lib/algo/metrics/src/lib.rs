//! brickie-metrics: per-function complexity and test-code detection from the tree-sitter syntax tree.
//!
//! Ported from the Python prototype (`cc.py`). Definitions:
//!
//! * **cognitive** (SonarSource, adapted to Rust): `+1 + nesting` for `if`, `match` (once, not per
//!   arm), `for`, `while`, `loop` and `let ... else`; `+1` for `else`; `+1` per sequence of like
//!   `&&` / `||` operators; `+1` for a labeled `break`/`continue`; `+1` per direct recursive call.
//!   Nesting grows inside `if`/`else`/`match`/loops/`let-else`/closures. `?` is free.
//! * **cyclomatic**: `1 + if + loops + (match arms - 1) + let-else + && + || + ?`.
//! * **nesting**: deepest nesting level of branching constructs and closures.
//! * **paths** (NPath variant): a block is `max(1, sum of its decisions)`; `if` is
//!   `B(then) + B(else or 1)` plus one per `&&`/`||` in the condition; `match` is the sum of its
//!   arms (a flat lookup table of three or more constant arms counts as one decision); a loop is
//!   `B(body) + 1`; `let ... else` is `B(else) + 1`; `?` adds one. Sequential decisions add,
//!   nested decisions combine through their enclosing block. Capped at 10^9.
//! * **params**: every parameter of the signature except `self` (clippy convention).
//!
//! Macro bodies (token trees) are not parsed. Nested `fn` items are measured separately.
//! A closure adds nesting but no cost itself, and a labeled `break`/`continue` costs a flat `+1`
//! (no nesting increment); both match the SonarSource definition.

mod analyze;
mod cognitive;
mod paths;
mod syntax;
mod testcfg;
mod tree;

pub use analyze::analyze;
pub use tree::fn_tree;

/// One node of a function's decision tree, nodes in pre-order.
#[derive(Debug, Clone, PartialEq)]
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

/// A function's decision tree with its headline metrics.
#[derive(Debug, Clone, PartialEq)]
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

/// A 0-based (row, UTF-8 byte column) position, as in SCIP.
pub type Pos = (u32, u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnMetrics {
    pub name: String,
    /// `Type::name` for methods, `name` for free functions.
    pub qname: String,
    /// 1-based first and last line.
    pub line: u32,
    pub end_line: u32,
    pub lines: u32,
    pub start: Pos,
    pub cognitive: u32,
    pub cyclomatic: u32,
    pub nesting: u32,
    pub paths: u64,
    pub params: u32,
    /// The function sits inside a test-gated item (`#[cfg(test)]`, `#[test]`, ...).
    pub test: bool,
}

#[derive(Debug, Default)]
pub struct Analysis {
    pub fns: Vec<FnMetrics>,
    /// Source ranges `[start, end)` of items gated as test code.
    pub test_ranges: Vec<(Pos, Pos)>,
    /// The file has an inner `#![cfg(test)]`-style attribute.
    pub whole_file_test: bool,
    /// `mod name;` declarations (file modules) with whether they are test-gated.
    pub mod_decls: Vec<(String, bool)>,
    /// Start of the trait name of every `impl Trait for Type`.
    pub impl_traits: Vec<Pos>,
}

impl Analysis {
    pub fn is_test_at(&self, pos: Pos) -> bool {
        self.whole_file_test || self.test_ranges.iter().any(|&(s, e)| s <= pos && pos < e)
    }
}
