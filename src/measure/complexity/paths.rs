//! NPath-style count of execution paths.

use super::cognitive::logic_count;
use super::syntax::{Ctx, LOOPS, field, kids, named};
use tree_sitter::Node;

const PATH_CAP: u64 = 1_000_000_000;

const CONST_ARM: [&str; 12] = [
    "string_literal",
    "raw_string_literal",
    "integer_literal",
    "float_literal",
    "boolean_literal",
    "char_literal",
    "identifier",
    "scoped_identifier",
    "unit_expression",
    "negative_literal",
    "field_expression",
    "reference_expression",
];

pub fn add(a: u64, b: u64) -> u64 {
    a.saturating_add(b).min(PATH_CAP)
}

/// B(n): paths of a branch body, at least 1.
pub fn block(cx: &Ctx, n: Node) -> u64 {
    paths(cx, n).max(1)
}

/// Decision contribution of `n`; 0 for straight-line code.
pub fn paths(cx: &Ctx, n: Node) -> u64 {
    let t = n.kind();
    match t {
        "function_item" | "attribute_item" => 0,
        "if_expression" => {
            let mut total = 0;
            if let Some(cond) = field(n, "condition") {
                total = add(logic_count(cx, cond), paths(cx, cond));
            }
            if let Some(c) = field(n, "consequence") {
                total = add(total, block(cx, c));
            }
            match kids(n).into_iter().rfind(|c| c.kind() == "else_clause") {
                None => add(total, 1),
                Some(alt) => add(total, named(alt).first().map_or(1, |i| block(cx, *i))),
            }
        }
        "match_expression" => {
            let mut total = field(n, "value").map_or(0, |v| paths(cx, v));
            let Some(body) = field(n, "body") else {
                return total;
            };
            if const_match(cx, body) {
                return add(total, 1); // lookup table: one decision, not N paths
            }
            for arm in named(body).into_iter().filter(|a| a.kind() == "match_arm") {
                let guard: u64 = named(arm)
                    .into_iter()
                    .filter(|c| c.kind() == "match_pattern")
                    .map(|c| add(logic_count(cx, c), paths(cx, c)))
                    .sum();
                total = match field(arm, "value") {
                    Some(v) => add(total, add(block(cx, v), guard)),
                    None => add(total, 1),
                };
            }
            total
        }
        _ if LOOPS.contains(&t) => {
            let body = field(n, "body");
            let mut total = add(1, body.map_or(1, |b| block(cx, b)));
            for c in named(n).into_iter().filter(|c| Some(*c) != body) {
                total = add(total, paths(cx, c));
                if t == "while_expression" {
                    total = add(total, logic_count(cx, c));
                }
            }
            total
        }
        "let_declaration" if field(n, "alternative").is_some() => {
            let alt = field(n, "alternative");
            let mut total = add(1, alt.map_or(1, |a| block(cx, a)));
            for c in named(n).into_iter().filter(|c| Some(*c) != alt) {
                total = add(total, paths(cx, c));
            }
            total
        }
        "try_expression" => named(n).into_iter().fold(1, |a, c| add(a, paths(cx, c))),
        _ => kids(n).into_iter().fold(0, |a, c| add(a, paths(cx, c))),
    }
}

/// Every arm (three or more) is an unguarded constant-like expression: a lookup table.
pub fn const_match(cx: &Ctx, body: Node) -> bool {
    let arms: Vec<Node> = named(body)
        .into_iter()
        .filter(|a| a.kind() == "match_arm")
        .collect();
    if arms.len() < 3 {
        return false;
    }
    arms.iter().all(|a| {
        let guarded = named(*a)
            .into_iter()
            .find(|c| c.kind() == "match_pattern")
            .is_some_and(|p| field(p, "condition").is_some());
        if guarded {
            return false;
        }
        let Some(v) = field(*a, "value") else {
            return false;
        };
        if v.kind() == "call_expression" {
            field(v, "arguments")
                .is_some_and(|args| named(args).iter().all(|x| CONST_ARM.contains(&x.kind())))
        } else {
            let _ = cx;
            CONST_ARM.contains(&v.kind())
        }
    })
}
