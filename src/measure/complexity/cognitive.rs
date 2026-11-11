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

pub fn walk(cx: &Ctx, n: Node, fa: &mut Acc, nest: u32, in_else_if: bool, parent_logic: bool) {
    let t = n.kind();
    if t == "function_item" || t == "attribute_item" {
        return; // nested fn: measured on its own
    }
    if t == "mod_item" && !leading_attrs(cx, n).iter().all(|a| !is_test_attr(a)) {
        return;
    }
    let branching = matches!(
        t,
        "if_expression" | "match_expression" | "closure_expression"
    ) || LOOPS.contains(&t)
        || (t == "let_declaration" && field(n, "alternative").is_some());
    if branching {
        fa.depth = fa.depth.max(nest + 1);
    }
    match t {
        "if_expression" => {
            fa.cyc += 1;
            fa.cog += if in_else_if { 1 } else { 1 + nest };
            let conseq = field(n, "consequence");
            for c in kids(n) {
                if c.kind() == "else_clause" {
                    let inner = named(c);
                    if let Some(first) = inner.first().filter(|x| x.kind() == "if_expression") {
                        walk(cx, *first, fa, nest, true, false);
                    } else {
                        fa.cog += 1;
                        for x in inner {
                            walk(cx, x, fa, nest + 1, false, false);
                        }
                    }
                } else {
                    let d = if Some(c) == conseq { nest + 1 } else { nest };
                    walk(cx, c, fa, d, false, false);
                }
            }
        }
        "match_expression" => {
            fa.cog += 1 + nest;
            let body = field(n, "body");
            let arms = body
                .map(|b| named(b).iter().filter(|c| c.kind() == "match_arm").count())
                .unwrap_or(0);
            fa.cyc += arms.saturating_sub(1) as u32;
            for c in kids(n) {
                let d = if Some(c) == body { nest + 1 } else { nest };
                walk(cx, c, fa, d, false, false);
            }
        }
        _ if LOOPS.contains(&t) => {
            fa.cyc += 1;
            fa.cog += 1 + nest;
            let body = field(n, "body");
            for c in kids(n) {
                let d = if Some(c) == body { nest + 1 } else { nest };
                walk(cx, c, fa, d, false, false);
            }
        }
        "let_declaration" if field(n, "alternative").is_some() => {
            fa.cyc += 1;
            fa.cog += 1 + nest;
            let alt = field(n, "alternative");
            for c in kids(n) {
                let d = if Some(c) == alt { nest + 1 } else { nest };
                walk(cx, c, fa, d, false, false);
            }
        }
        "closure_expression" => {
            for c in kids(n) {
                walk(cx, c, fa, nest + 1, false, false);
            }
        }
        _ if is_logic(cx, n) => {
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
        _ => {
            match t {
                "try_expression" => fa.cyc += 1,
                "break_expression" | "continue_expression" => {
                    if kids(n).iter().any(|c| c.kind() == "label") {
                        fa.cog += 1;
                    }
                }
                "call_expression" => {
                    if is_recursive_call(cx, n, &fa.name) {
                        fa.cog += 1;
                    }
                }
                _ => {}
            }
            for c in kids(n) {
                walk(cx, c, fa, nest, false, false);
            }
        }
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
