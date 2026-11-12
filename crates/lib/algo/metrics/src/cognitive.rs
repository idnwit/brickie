//! Cognitive complexity, cyclomatic complexity and nesting of one function body.

use super::syntax::{Ctx, LOOPS, field, is_logic, kids, leading_attrs, logic_ops, named};
use super::testcfg::is_test_attr;
use tree_sitter::Node;

#[derive(Default)]
pub struct Acc {
    pub name: String,
    pub cog: u32,
    pub cyc: u32,
    pub depth: u32,
}

/// Walks `n` and adds its cognitive/cyclomatic cost and nesting depth to `fa`.
///
/// Flow: skip nodes measured elsewhere, dispatch on the node kind. Branching nodes (`if`,
/// `match`, closures, loops, `let ... else`) fall through to record depth `nest + 1`; the others
/// return early. `nest` is the current nesting level; `in_else_if` marks an `if` that continues
/// an `else if` chain; `parent_logic` marks a `&&`/`||` node whose parent is one too (its chain
/// is already counted).
pub fn walk(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32, in_else_if: bool, parent_logic: bool) {
    if skip(cx, n) {
        return;
    }
    let t = n.kind();
    match t {
        "if_expression" => walk_if(cx, n, fa, nest, in_else_if),
        "match_expression" => walk_match(cx, n, fa, nest),
        "let_declaration" if field(n, "alternative").is_some() => walk_nested(cx, n, fa, nest),
        _ if LOOPS.contains(&t) => walk_nested(cx, n, fa, nest),
        // Closures add nesting but no cost.
        "closure_expression" => walk_plain(cx, n, fa, nest + 1),
        _ if is_logic(cx, n) => return walk_logic(cx, n, fa, nest, parent_logic),
        _ => {
            leaf_cost(cx, n, fa);
            return walk_plain(cx, n, fa, nest);
        }
    }
    fa.depth = fa.depth.max(nest + 1);
}

/// True for nodes that are measured on their own or not at all: nested `fn` items, attribute
/// items, and `mod` items carrying a test attribute.
fn skip(cx: &Ctx, n: Node) -> bool {
    match n.kind() {
        "function_item" | "attribute_item" => true,
        "mod_item" => leading_attrs(cx, n).into_iter().any(is_test_attr),
        _ => false,
    }
}

/// Nesting level for child `c` of `n`: `nest + 1` when `c` is `n`'s `body`, `consequence` or
/// `alternative` field, otherwise `nest`.
fn child_nest(n: Node, c: Node, nest: u32) -> u32 {
    let in_block = ["body", "consequence", "alternative"]
        .iter()
        .any(|f| field(n, f) == Some(c));
    if in_block { nest + 1 } else { nest }
}

/// Walks every child of `n` with the nesting from [`child_nest`]; an `else_clause` goes through
/// [`walk_else`].
fn walk_blocks(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32) {
    for c in kids(n) {
        if c.kind() == "else_clause" {
            walk_else(cx, c, fa, nest);
        } else {
            walk(cx, c, fa, child_nest(n, c, nest), false, false);
        }
    }
}

/// Walks every child of `n` at the same nesting level.
fn walk_plain(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32) {
    for c in kids(n) {
        walk(cx, c, fa, nest, false, false);
    }
}

/// `if`: cyclomatic +1; cognitive `1 + nest`, or just 1 when continuing an `else if` chain. An
/// `else if` is walked at the same nesting; a plain `else` costs 1 and nests its content.
fn walk_if(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32, in_else_if: bool) {
    fa.cyc += 1;
    fa.cog += if in_else_if { 1 } else { 1 + nest };
    walk_blocks(cx, n, fa, nest);
}

/// An `else_clause`: continues an `else if` chain when its first named child is an `if`,
/// otherwise costs 1 and walks its content one level deeper.
fn walk_else(cx: &Ctx, clause: Node, fa: &mut Acc, nest: u32) {
    let inner = named(clause);
    if let Some(first) = inner.first().filter(|x| x.kind() == "if_expression") {
        walk(cx, *first, fa, nest, true, false);
    } else {
        fa.cog += 1;
        for x in inner {
            walk(cx, x, fa, nest + 1, false, false);
        }
    }
}

/// `match`: cognitive `1 + nest` once (not per arm); cyclomatic `arms - 1`; the body nests.
fn walk_match(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32) {
    fa.cog += 1 + nest;
    let arms = field(n, "body").map_or(0, |b| {
        named(b).iter().filter(|c| c.kind() == "match_arm").count()
    });
    fa.cyc += arms.saturating_sub(1) as u32;
    walk_blocks(cx, n, fa, nest);
}

/// Loops (`for`, `while`, `loop`) and `let ... else`: cyclomatic +1, cognitive `1 + nest`; the
/// loop body or the `else` block nests.
fn walk_nested(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32) {
    fa.cyc += 1;
    fa.cog += 1 + nest;
    walk_blocks(cx, n, fa, nest);
}

/// `&&` / `||`: the outermost node of a chain costs cyclomatic +1 per operator and cognitive
/// `1 +` the number of operator switches; inner chain nodes (`parent_logic`) add nothing. Children
/// are walked at the same nesting.
fn walk_logic(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32, parent_logic: bool) {
    if !parent_logic {
        let mut ops = Vec::new();
        logic_ops(cx, n, &mut ops);
        fa.cyc += ops.len() as u32;
        fa.cog += 1 + ops.windows(2).filter(|w| w[0] != w[1]).count() as u32;
    }
    for c in kids(n) {
        walk(cx, c, fa, nest, false, is_logic(cx, c));
    }
}

/// Cost of a node that does not nest: `?` is cyclomatic +1; a labeled `break`/`continue` and a
/// direct recursive call are cognitive +1; anything else is free.
fn leaf_cost(cx: &Ctx, n: Node, fa: &mut Acc) {
    match n.kind() {
        "try_expression" => fa.cyc += 1,
        "break_expression" | "continue_expression"
            if kids(n).iter().any(|c| c.kind() == "label") =>
        {
            fa.cog += 1;
        }
        "call_expression" if is_recursive_call(cx, n, &fa.name) => fa.cog += 1,
        _ => {}
    }
}

/// `name()`, `Self::name()` or `self.name()`.
pub fn is_recursive_call(cx: &Ctx, call: Node, name: &str) -> bool {
    let Some(f) = field(call, "function") else {
        return false;
    };
    match f.kind() {
        "identifier" => cx.txt(f) == name,
        "scoped_identifier" => cx.txt(f).strip_prefix("Self::") == Some(name),
        "field_expression" => {
            field(f, "value").is_some_and(|v| cx.txt(v) == "self")
                && field(f, "field").is_some_and(|x| cx.txt(x) == name)
        }
        _ => false,
    }
}

pub fn logic_count(cx: &Ctx, n: Node) -> u64 {
    // Counts logic operators in `n` without entering blocks, match bodies or closures.
    let mut total = u64::from(is_logic(cx, n));
    for c in kids(n) {
        if !matches!(c.kind(), "block" | "match_block" | "closure_expression") {
            total += logic_count(cx, c);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Ctx;
    use tree_sitter::{Parser, Tree};

    fn parse(src: &str) -> Tree {
        let mut p = Parser::new();
        p.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        p.parse(src, None).unwrap()
    }

    /// Pre-order search for the `nth` (0-based) node of `kind`.
    fn find<'t>(n: Node<'t>, kind: &str, nth: &mut usize) -> Option<Node<'t>> {
        if n.kind() == kind {
            if *nth == 0 {
                return Some(n);
            }
            *nth -= 1;
        }
        kids(n).into_iter().find_map(|c| find(c, kind, nth))
    }

    /// Parses `src`, takes the `nth` node of `kind`, runs `f` on it and returns (cog, cyc, depth).
    fn run(
        src: &str,
        kind: &str,
        nth: usize,
        f: impl for<'t> Fn(&Ctx, Node<'t>, &mut Acc),
    ) -> (u32, u32, u32) {
        let tree = parse(src);
        let cx = Ctx {
            src: src.as_bytes(),
        };
        let n = find(tree.root_node(), kind, &mut { nth }).expect("node");
        let mut fa = Acc {
            name: "f".into(),
            ..Acc::default()
        };
        f(&cx, n, &mut fa);
        (fa.cog, fa.cyc, fa.depth)
    }

    #[test]
    fn skip_kinds() {
        let src = "fn f() { fn g() {} #[cfg(test)] mod t {} mod u {} #[derive(A)] mod v {} 1; }";
        let tree = parse(src);
        let cx = Ctx {
            src: src.as_bytes(),
        };
        let sk =
            |kind: &str, nth: usize| skip(&cx, find(tree.root_node(), kind, &mut { nth }).unwrap());
        assert!(sk("function_item", 0));
        assert!(sk("attribute_item", 0));
        assert!(sk("mod_item", 0), "cfg(test) mod");
        assert!(!sk("mod_item", 1), "plain mod");
        assert!(!sk("mod_item", 2), "mod with non-test attribute");
        assert!(!sk("integer_literal", 0));
    }

    #[test]
    fn child_nest_fields() {
        let src = "fn f(a: bool) { if a { 1 } else { 2 }; for i in x {}; let Some(y) = z else { return }; }";
        let tree = parse(src);
        let root = tree.root_node();
        let at = |kind: &str| find(root, kind, &mut { 0 }).unwrap();
        let iff = at("if_expression");
        let conseq = field(iff, "consequence").unwrap();
        let cond = field(iff, "condition").unwrap();
        assert_eq!(child_nest(iff, conseq, 3), 4);
        assert_eq!(child_nest(iff, cond, 3), 3);
        let fr = at("for_expression");
        assert_eq!(child_nest(fr, field(fr, "body").unwrap(), 0), 1);
        assert_eq!(child_nest(fr, field(fr, "value").unwrap(), 0), 0);
        let le = at("let_declaration");
        let le = if field(le, "alternative").is_some() {
            le
        } else {
            find(root, "let_declaration", &mut { 1 }).unwrap()
        };
        assert_eq!(child_nest(le, field(le, "alternative").unwrap(), 2), 3);
        assert_eq!(child_nest(le, field(le, "pattern").unwrap(), 2), 2);
    }

    #[test]
    fn leaf_costs() {
        let w = |cx: &Ctx, n: Node, fa: &mut Acc| leaf_cost(cx, n, fa);
        assert_eq!(run("fn f() { g()?; }", "try_expression", 0, w), (0, 1, 0));
        assert_eq!(
            run("fn f() { loop { break 'a; } }", "break_expression", 0, w).0,
            1
        );
        assert_eq!(
            run("fn f() { loop { break; } }", "break_expression", 0, w).0,
            0
        );
        assert_eq!(
            run(
                "fn f() { loop { continue 'a; } }",
                "continue_expression",
                0,
                w
            )
            .0,
            1
        );
        assert_eq!(
            run("fn f() { loop { continue; } }", "continue_expression", 0, w).0,
            0
        );
        let call = |s: &str| run(s, "call_expression", 0, w).0;
        assert_eq!(call("fn f() { f(); }"), 1);
        assert_eq!(call("fn f() { Self::f(); }"), 1);
        assert_eq!(call("fn f() { g(); }"), 0);
        assert_eq!(call("fn f() { Other::f(); }"), 0);
        assert_eq!(call("fn f() { (x)(); }"), 0);
        let m = |s: &str| run(s, "call_expression", 0, w).0;
        assert_eq!(m("fn f() { self.f(); }"), 1);
        assert_eq!(m("fn f() { other.f(); }"), 0);
        assert_eq!(m("fn f() { self.g(); }"), 0);
        assert_eq!(
            run("fn f() { 1 + 2; }", "binary_expression", 0, w),
            (0, 0, 0)
        );
    }
}
