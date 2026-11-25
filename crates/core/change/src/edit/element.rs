//! The elements of a config file by identity, as canonical TOML: what the semantic diff
//! compares.

use super::find::rule_label;
use super::rules::{COMPLEXITY, LINKS, RULES};
use super::table::{aot, canon};
use toml_edit::{DocumentMut, Item};

/// `(label, canonical TOML)` of every element of `doc`: `brick <name>`, `rule <from> -> <to>
/// [kinds] (scope)`, `limit <bricks>`, `exports`, `deps`, `include`. A repeated label gets
/// `#2`, `#3`.
pub fn elements(doc: &DocumentMut) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |label: String, c: String| {
        let n = out
            .iter()
            .filter(|(l, _)| l.split(" #").next() == Some(&label))
            .count();
        let label = match n {
            0 => label,
            n => format!("{label} #{}", n + 1),
        };
        out.push((label, c));
    };
    for t in aot(doc, &["brick"]).into_iter().flat_map(|a| a.iter()) {
        let name = t.get("name").and_then(Item::as_str).unwrap_or_default();
        push(format!("brick {name}"), canon(t.iter()));
    }
    for t in aot(doc, &["group"]).into_iter().flat_map(|a| a.iter()) {
        let name = t.get("name").and_then(Item::as_str).unwrap_or_default();
        push(format!("group {name}"), canon(t.iter()));
    }
    for (noun, table) in [("rule", RULES), ("link", LINKS)] {
        for t in aot(doc, table).into_iter().flat_map(|a| a.iter()) {
            push(rule_label(noun, t), canon(t.iter()));
        }
    }
    for t in aot(doc, COMPLEXITY).into_iter().flat_map(|a| a.iter()) {
        let sel = t.get("bricks").and_then(Item::as_str).unwrap_or("*");
        push(format!("limit {sel}"), canon(t.iter()));
    }
    for k in ["exports", "include"] {
        if let Some(i) = doc.get(k) {
            push(k.into(), canon(std::iter::once((k, i))));
        }
    }
    out
}
