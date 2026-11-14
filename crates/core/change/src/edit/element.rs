//! The elements of a config file by identity, as canonical TOML: what the semantic diff
//! compares.

use super::find::rule_label;
use super::rules::{COMPLEXITY, RULES};
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
    for t in aot(doc, RULES).into_iter().flat_map(|a| a.iter()) {
        push(rule_label(t), canon(t.iter()));
    }
    for t in aot(doc, COMPLEXITY).into_iter().flat_map(|a| a.iter()) {
        let sel = t.get("bricks").and_then(Item::as_str).unwrap_or("*");
        push(format!("limit {sel}"), canon(t.iter()));
    }
    if let Some(deps) = doc.get("deps").and_then(Item::as_table) {
        let items = deps
            .iter()
            .filter(|(k, i)| *k != "rule" && !i.is_array_of_tables());
        let c = canon(items);
        if !c.is_empty() {
            push("deps".into(), c);
        }
    }
    for k in ["exports", "include"] {
        if let Some(i) = doc.get(k) {
            push(k.into(), canon(std::iter::once((k, i))));
        }
    }
    out
}
