//! Declared-interface candidates: every non-test `pub` / `pub(crate)` item, with its signature,
//! doc and whether it is reachable as public API from a library crate root.

use super::complexity::Analysis;
use super::edges::symbol_name;
use crate::scip::Graph;
use crate::scip::modules::ModuleIds;
use ::scip::types::descriptor::Suffix;
use brickie_facts::{FileData, Item};
use std::collections::{BTreeMap, HashSet};

/// `(vis, keyword)` of a signature: `pub fn f()` -> `("pub", "fn")`. `None` for private
/// items, `pub(super)` / `pub(in ..)`, and signatures without an item keyword (fields).
pub fn parse_sig(sig: &str) -> Option<(&'static str, &'static str)> {
    let (vis, rest) = if let Some(r) = sig.strip_prefix("pub(crate) ") {
        ("pub(crate)", r)
    } else if let Some(r) = sig.strip_prefix("pub ") {
        ("pub", r)
    } else {
        return None;
    };
    let toks: Vec<&str> = rest.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        let kw = match *t {
            "fn" => "fn",
            "struct" => "struct",
            "enum" => "enum",
            "union" => "union",
            "trait" => "trait",
            "type" => "type",
            "static" => "static",
            "mod" => "mod",
            "const" if toks.get(i + 1).is_some_and(|n| !is_qualifier(n)) => "const",
            q if is_qualifier(q) => continue,
            _ => return None,
        };
        return Some((vis, kw));
    }
    None
}

fn is_qualifier(t: &str) -> bool {
    matches!(
        t,
        "const" | "async" | "unsafe" | "extern" | "auto" | "default" | "fn"
    ) || t.starts_with('"')
}

/// Signature on one line, whitespace collapsed.
pub fn one_line(sig: &str) -> String {
    sig.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// First paragraph of a doc comment, on one line; rust-analyzer's alias banner is dropped.
pub fn first_paragraph(doc: &str) -> String {
    doc.split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with("*This is the documentation for*"))
        .map(one_line)
        .unwrap_or_default()
}

/// How a symbol sits in its owner: a free item, an inherent impl item (`Some(type name)`), or
/// a trait member / trait impl item (skipped).
enum Owner {
    Free,
    Inherent(String),
    Skip,
}

fn owner(scip: &str, traits: &HashSet<String>) -> Owner {
    let Ok(s) = ::scip::symbol::parse_symbol(scip) else {
        return Owner::Skip;
    };
    let d = &s.descriptors;
    let suffix = |i: usize| d[i].suffix.enum_value_or_default();
    let is_impl = |x: &::scip::types::Descriptor| {
        x.name == "impl" && x.suffix.enum_value_or_default() == Suffix::Type
    };
    if let Some(i) = d.iter().position(is_impl) {
        let params: Vec<&str> = d[i + 1..]
            .iter()
            .take_while(|x| x.suffix.enum_value_or_default() == Suffix::TypeParameter)
            .map(|x| x.name.as_str())
            .collect();
        return match params.as_slice() {
            [ty] => Owner::Inherent(ty.to_string()),
            _ => Owner::Skip,
        };
    }
    let n = d.len();
    if n >= 2 && suffix(n - 2) == Suffix::Type {
        let mut parent = s.clone();
        parent.descriptors.truncate(n - 1);
        if traits.contains(&::scip::symbol::format_symbol(parent)) {
            return Owner::Skip;
        }
        return Owner::Inherent(d[n - 2].name.clone());
    }
    Owner::Free
}

/// Fills `items` of every compiled file.
pub fn add_items(
    files: &mut BTreeMap<String, FileData>,
    g: &Graph,
    ids: &ModuleIds,
    crate_ids: &[String],
    analyses: &BTreeMap<String, Analysis>,
    test_set: &HashSet<String>,
) {
    let traits: HashSet<String> = g
        .symbols
        .iter()
        .filter(|s| s.signature.split_whitespace().any(|t| t == "trait"))
        .map(|s| s.scip.clone())
        .collect();
    let reach = super::reach::Reach::new(g, ids, crate_ids);
    for (i, s) in g.symbols.iter().enumerate() {
        if s.module_path.is_some() {
            continue;
        }
        let Some((vis, kw)) = parse_sig(&s.signature) else {
            continue;
        };
        let f = &g.files[s.file];
        if test_set.contains(&f.path) || analyses[&f.path].is_test_at((s.range.0, s.range.1)) {
            continue;
        }
        let Some(module) = s.module.and_then(|m| ids.get(&m)) else {
            continue;
        };
        let (kind, name, exported) = match owner(&s.scip, &traits) {
            Owner::Skip => continue,
            Owner::Free => (kw, s.name.clone(), vis == "pub" && reach.item(i, module)),
            Owner::Inherent(ty) => (
                if kw == "fn" { "method" } else { kw },
                format!("{ty}::{}", s.name),
                vis == "pub" && reach.assoc(module, &ty),
            ),
        };
        if let Some(fd) = files.get_mut(&f.path) {
            fd.items.push(Item {
                symbol: symbol_name(&s.scip, &s.name),
                module: module.clone(),
                kind: kind.into(),
                name,
                signature: one_line(&s.signature),
                doc: first_paragraph(&s.doc),
                line: s.range.0 + 1,
                vis: vis.into(),
                visible_in: reach.visible_in(i, module, vis),
                exported,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures() {
        assert_eq!(parse_sig("pub fn f(x: u32)"), Some(("pub", "fn")));
        assert_eq!(parse_sig("pub const fn f()"), Some(("pub", "fn")));
        assert_eq!(
            parse_sig("pub(crate) const X: u32"),
            Some(("pub(crate)", "const"))
        );
        assert_eq!(parse_sig("pub unsafe trait T"), Some(("pub", "trait")));
        assert_eq!(parse_sig("pub(super) fn f()"), None);
        assert_eq!(parse_sig("fn f()"), None);
        assert_eq!(parse_sig("pub name: String"), None);
    }

    #[test]
    fn docs() {
        assert_eq!(first_paragraph("One\nline.\n\nMore."), "One line.");
        assert_eq!(
            first_paragraph("*This is the documentation for* `pub struct A`\n\nBody."),
            "Body."
        );
    }

    #[test]
    fn owners() {
        let t: HashSet<String> = ["rust-analyzer cargo p 0.1.0 m/T#".to_string()].into();
        let o = |s: &str| match owner(s, &t) {
            Owner::Free => "free".to_string(),
            Owner::Inherent(x) => x,
            Owner::Skip => "skip".into(),
        };
        assert_eq!(o("rust-analyzer cargo p 0.1.0 m/f()."), "free");
        assert_eq!(o("rust-analyzer cargo p 0.1.0 m/impl#[A]new()."), "A");
        assert_eq!(o("rust-analyzer cargo p 0.1.0 m/impl#[A][T]f()."), "skip");
        assert_eq!(o("rust-analyzer cargo p 0.1.0 m/T#f()."), "skip");
        assert_eq!(o("rust-analyzer cargo p 0.1.0 m/A#X."), "A");
    }
}
