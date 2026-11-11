//! Walks a parsed file and measures every function.

use super::cognitive::{Acc, walk};
use super::paths::block;
use super::syntax::{Ctx, TRIVIA, field, kids, leading_attrs, named, pos};
use super::testcfg::{is_test_attr, is_test_cfg};
use super::{Analysis, FnMetrics};
use tree_sitter::{Node, Parser};

fn owner_prefix(cx: &Ctx, f: Node) -> String {
    let mut p = f.parent();
    while let Some(x) = p {
        match x.kind() {
            "impl_item" => {
                return field(x, "type").map_or(String::new(), |t| format!("{}::", cx.txt(t)));
            }
            "trait_item" => {
                return field(x, "name").map_or(String::new(), |t| format!("{}::", cx.txt(t)));
            }
            _ => {}
        }
        p = x.parent();
    }
    String::new()
}

pub fn measure_fn(cx: &Ctx, f: Node, test: bool) -> Option<FnMetrics> {
    let body = field(f, "body")?;
    let name = field(f, "name").map_or("", |n| cx.txt(n)).to_string();
    let mut acc = Acc {
        name: name.clone(),
        cyc: 1,
        ..Acc::default()
    };
    walk(cx, body, &mut acc, 0, false, false);
    let params = field(f, "parameters").map_or(0, |p| {
        named(p)
            .iter()
            .filter(|c| !TRIVIA.contains(&c.kind()) && c.kind() != "self_parameter")
            .count() as u32
    });
    let (s, e) = (f.start_position(), f.end_position());
    Some(FnMetrics {
        qname: format!("{}{name}", owner_prefix(cx, f)),
        name,
        line: s.row as u32 + 1,
        end_line: e.row as u32 + 1,
        lines: (e.row - s.row) as u32 + 1,
        start: pos(s),
        cognitive: acc.cog,
        cyclomatic: acc.cyc,
        nesting: acc.depth,
        paths: block(cx, body),
        params,
        test,
    })
}

/// Name node of the trait in `impl Trait for Type` (`a::B<T>` -> `B`).
fn trait_name_node(t: Node) -> Option<Node> {
    match t.kind() {
        "generic_type" => field(t, "type").and_then(trait_name_node),
        "scoped_type_identifier" => field(t, "name"),
        "type_identifier" => Some(t),
        _ => None,
    }
}

fn collect(cx: &Ctx, n: Node, in_test: bool, out: &mut Analysis) {
    for c in kids(n) {
        let attr_test = leading_attrs(cx, c).iter().any(|a| is_test_attr(a));
        let test = in_test || attr_test;
        if attr_test && !in_test && !TRIVIA.contains(&c.kind()) {
            out.test_ranges
                .push((pos(c.start_position()), pos(c.end_position())));
        }
        match c.kind() {
            "function_item" => {
                if let Some(m) = measure_fn(cx, c, test) {
                    out.fns.push(m);
                }
            }
            "mod_item" if field(c, "body").is_none() => {
                if let Some(nm) = field(c, "name") {
                    out.mod_decls.push((cx.txt(nm).to_string(), test));
                }
            }
            "impl_item" => {
                if let Some(p) = field(c, "trait").and_then(trait_name_node) {
                    out.impl_traits.push(pos(p.start_position()));
                }
            }
            "inner_attribute_item" if is_test_cfg(cx.txt(c)) => out.whole_file_test = true,
            _ => {}
        }
        collect(cx, c, test, out);
    }
}

/// Parse `src` and measure every function with a body plus test ranges and impl-trait positions.
pub fn analyze(src: &str) -> Analysis {
    let mut out = Analysis::default();
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return out;
    }
    let Some(tree) = parser.parse(src, None) else {
        return out;
    };
    collect(
        &Ctx {
            src: src.as_bytes(),
        },
        tree.root_node(),
        false,
        &mut out,
    );
    out
}
