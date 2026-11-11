//! Decision tree of one function (the vertical complexity tree shown next to the source).

use super::analyze::measure_fn;
use super::cognitive::logic_count;
use super::paths::{add, block, paths};
use super::syntax::{Ctx, LOOPS, field, kids, named};
use super::{FnTree, TreeNode};
use tree_sitter::{Node, Parser};

/// A node of the decision tree with its children, before flattening.
struct Tn {
    node: TreeNode,
    kids: Vec<Tn>,
}

fn snip(cx: &Ctx, n: Option<Node>, k: usize) -> String {
    let s = n.map_or(String::new(), |n| {
        cx.txt(n).split_whitespace().collect::<Vec<_>>().join(" ")
    });
    if s.chars().count() <= k {
        s
    } else {
        format!("{}…", s.chars().take(k - 1).collect::<String>())
    }
}

fn mk(kind: &str, n: Node, label: String, paths: u64, depth: u32, kids: Vec<Tn>) -> Tn {
    Tn {
        node: TreeNode {
            kind: kind.to_string(),
            label,
            line: n.start_position().row as u32 + 1,
            end_line: n.end_position().row as u32 + 1,
            paths,
            depth,
        },
        kids,
    }
}

/// `_opnodes`: a `&&`/`||` chain in a condition or pattern.
fn opnodes(cx: &Ctx, cond: Option<Node>, depth: u32) -> Vec<Tn> {
    let Some(c) = cond else { return vec![] };
    let k = logic_count(cx, c);
    if k == 0 {
        return vec![];
    }
    vec![mk("cond_op", c, snip(cx, Some(c), 40), k, depth, vec![])]
}

fn tree_kids(cx: &Ctx, n: Option<Node>, depth: u32) -> Vec<Tn> {
    n.map_or(vec![], |n| {
        kids(n)
            .into_iter()
            .flat_map(|c| tree(cx, c, depth))
            .collect()
    })
}

/// `_tree`.
fn tree(cx: &Ctx, n: Node, depth: u32) -> Vec<Tn> {
    match n.kind() {
        "function_item" | "attribute_item" => vec![],
        "if_expression" => {
            let cond = field(n, "condition");
            let mut ch = opnodes(cx, cond, depth + 1);
            ch.extend(tree_kids(cx, cond, depth + 1));
            ch.extend(tree_kids(cx, field(n, "consequence"), depth + 1));
            for c in kids(n).into_iter().filter(|c| c.kind() == "else_clause") {
                let inner = named(c).first().copied();
                let sub = match inner {
                    Some(i) if i.kind() == "if_expression" => tree(cx, i, depth + 1),
                    other => tree_kids(cx, other, depth + 1),
                };
                let p = inner.map_or(1, |i| block(cx, i));
                ch.push(mk("else", c, "else".into(), p, depth + 1, sub));
            }
            vec![mk("if", n, snip(cx, cond, 40), paths(cx, n), depth, ch)]
        }
        "match_expression" => {
            let mut ch = Vec::new();
            let body = field(n, "body");
            for arm in body.map(named).unwrap_or_default() {
                if arm.kind() != "match_arm" {
                    continue;
                }
                let v = field(arm, "value");
                let pat = named(arm).into_iter().find(|c| c.kind() == "match_pattern");
                let g: u64 = named(arm)
                    .into_iter()
                    .filter(|c| c.kind() == "match_pattern")
                    .map(|c| add(logic_count(cx, c), paths(cx, c)))
                    .sum();
                let mut sub = opnodes(cx, pat, depth + 2);
                match v {
                    Some(v) if v.kind() != "block" => sub.extend(tree(cx, v, depth + 2)),
                    other => sub.extend(tree_kids(cx, other, depth + 2)),
                }
                let p = v.map_or(1, |v| add(block(cx, v), g));
                ch.push(mk("arm", arm, snip(cx, pat, 40), p, depth + 1, sub));
            }
            let mut out = tree_kids(cx, field(n, "value"), depth);
            out.push(mk(
                "match",
                n,
                snip(cx, field(n, "value"), 40),
                paths(cx, n),
                depth,
                ch,
            ));
            out
        }
        t if LOOPS.contains(&t) => {
            let bd = field(n, "body");
            let hdr = snip(
                cx,
                field(n, "condition")
                    .or_else(|| field(n, "value"))
                    .or_else(|| field(n, "pattern")),
                40,
            );
            let mut ch: Vec<Tn> = named(n)
                .into_iter()
                .filter(|c| Some(*c) != bd)
                .flat_map(|c| opnodes(cx, Some(c), depth + 1))
                .collect();
            ch.extend(tree_kids(cx, bd, depth + 1));
            let word = t.split('_').next().unwrap_or("loop");
            vec![mk(
                "loop",
                n,
                format!("{word} {hdr}"),
                paths(cx, n),
                depth,
                ch,
            )]
        }
        "let_declaration" if field(n, "alternative").is_some() => {
            let alt = field(n, "alternative");
            vec![mk(
                "if",
                n,
                format!("let-else {}", snip(cx, field(n, "pattern"), 28)),
                paths(cx, n),
                depth,
                tree_kids(cx, alt, depth + 1),
            )]
        }
        "closure_expression" => {
            let body = field(n, "body");
            let label = snip(cx, field(n, "parameters"), 30);
            vec![mk(
                "closure",
                n,
                if label.is_empty() { "||".into() } else { label },
                body.map_or(1, |b| block(cx, b)),
                depth,
                body.map_or(vec![], |b| tree(cx, b, depth + 1)),
            )]
        }
        "try_expression" => {
            let mut out = tree_kids(cx, Some(n), depth);
            out.push(mk("qmark", n, snip(cx, Some(n), 40), 1, depth, vec![]));
            out
        }
        _ => tree_kids(cx, Some(n), depth),
    }
}

fn flatten(t: Tn, out: &mut Vec<TreeNode>) {
    out.push(t.node);
    for k in t.kids {
        flatten(k, out);
    }
}

/// `fn_tree_source`: the decision tree of the innermost function with a body containing the
/// 1-based `line`, nodes in pre-order (a node's `depth` is its nesting level, children deeper).
pub fn fn_tree(src: &str, line: u32) -> Option<FnTree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .ok()?;
    let parsed = parser.parse(src, None)?;
    let cx = Ctx {
        src: src.as_bytes(),
    };
    fn find<'t>(n: Node<'t>, line: u32, best: &mut Option<Node<'t>>) {
        for c in kids(n) {
            let (s, e) = (
                c.start_position().row as u32 + 1,
                c.end_position().row as u32 + 1,
            );
            if s <= line && line <= e {
                if c.kind() == "function_item" && field(c, "body").is_some() {
                    *best = Some(c);
                }
                find(c, line, best);
            }
        }
    }
    let mut best = None;
    find(parsed.root_node(), line, &mut best);
    let f = best?;
    let m = measure_fn(&cx, f, false)?;
    let body = field(f, "body")?;
    let mut nodes = Vec::new();
    for t in tree(&cx, body, 1) {
        flatten(t, &mut nodes);
    }
    Some(FnTree {
        label: m.qname,
        line: m.line,
        end_line: m.end_line,
        lines: m.lines,
        paths: m.paths,
        cognitive: m.cognitive,
        cyclomatic: m.cyclomatic,
        max_depth: m.nesting,
        nodes,
    })
}
