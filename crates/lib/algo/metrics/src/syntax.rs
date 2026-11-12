//! Shared tree-sitter helpers.

use super::Pos;
use tree_sitter::Node;

pub const LOOPS: [&str; 3] = ["for_expression", "while_expression", "loop_expression"];
pub const TRIVIA: [&str; 3] = ["attribute_item", "line_comment", "block_comment"];

pub struct Ctx<'a> {
    pub src: &'a [u8],
}

impl Ctx<'_> {
    pub fn txt(&self, n: Node) -> &str {
        n.utf8_text(self.src).unwrap_or("")
    }
}

pub fn kids(n: Node) -> Vec<Node> {
    let mut c = n.walk();
    n.children(&mut c).collect()
}

pub fn named(n: Node) -> Vec<Node> {
    let mut c = n.walk();
    n.named_children(&mut c).collect()
}

pub fn pos(p: tree_sitter::Point) -> Pos {
    (p.row as u32, p.column as u32)
}

pub fn field<'t>(n: Node<'t>, name: &str) -> Option<Node<'t>> {
    n.child_by_field_name(name)
}

/// Attribute items directly preceding `n` (skipping comments), as text.
pub fn leading_attrs<'a>(cx: &'a Ctx, n: Node) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut s = n.prev_named_sibling();
    while let Some(x) = s {
        if !TRIVIA.contains(&x.kind()) {
            break;
        }
        if x.kind() == "attribute_item" {
            out.push(cx.txt(x));
        }
        s = x.prev_named_sibling();
    }
    out
}

pub fn is_logic(cx: &Ctx, n: Node) -> bool {
    n.kind() == "binary_expression"
        && field(n, "operator").is_some_and(|o| matches!(cx.txt(o), "&&" | "||"))
}

/// In-order operator list of a `&&` / `||` chain (parentheses end the chain).
pub fn logic_ops<'a>(cx: &'a Ctx, n: Node, out: &mut Vec<&'a str>) {
    if is_logic(cx, n) {
        if let Some(l) = field(n, "left") {
            logic_ops(cx, l, out);
        }
        if let Some(o) = field(n, "operator") {
            out.push(cx.txt(o));
        }
        if let Some(r) = field(n, "right") {
            logic_ops(cx, r, out);
        }
    }
}
