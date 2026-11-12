//! Declared-interface candidates: every non-test `pub` / `pub(crate)` item, with its signature,
//! doc and whether it is reachable as public API from a library crate root.

use super::complexity::Analysis;
use super::edges::symbol_name;
use super::reach::Reach;
use crate::scip::modules::ModuleIds;
use crate::scip::{Graph, SymbolRow};
use ::scip::types::descriptor::Suffix;
use brickie_facts::{FileData, Item};
use std::collections::{BTreeMap, HashSet};

/// `(vis, keyword)` of a signature: `pub fn f()` -> `("pub", "fn")`. `None` for private
/// items, `pub(super)` / `pub(in ..)`, and signatures without an item keyword (fields).
pub fn parse_sig(sig: &str) -> Option<(&'static str, &'static str)> {
    let (vis, rest) = match sig.strip_prefix("pub(crate) ") {
        Some(r) => ("pub(crate)", r),
        None => ("pub", sig.strip_prefix("pub ")?),
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

/// What the item pass needs to judge one symbol: the graph, its module ids, which files and
/// ranges are test code, the trait symbols and public reachability.
struct Items<'a> {
    g: &'a Graph,
    ids: &'a ModuleIds,
    analyses: &'a BTreeMap<String, Analysis>,
    test_set: &'a HashSet<String>,
    traits: HashSet<String>,
    reach: Reach,
}

impl<'a> Items<'a> {
    fn new(
        g: &'a Graph,
        ids: &'a ModuleIds,
        crate_ids: &[String],
        analyses: &'a BTreeMap<String, Analysis>,
        test_set: &'a HashSet<String>,
    ) -> Self {
        let traits = g
            .symbols
            .iter()
            .filter(|s| s.signature.split_whitespace().any(|t| t == "trait"))
            .map(|s| s.scip.clone())
            .collect();
        Items {
            g,
            ids,
            analyses,
            test_set,
            traits,
            reach: Reach::new(g, ids, crate_ids),
        }
    }

    fn is_test(&self, path: &str, s: &SymbolRow) -> bool {
        self.test_set.contains(path) || self.analyses[path].is_test_at((s.range.0, s.range.1))
    }

    /// `(kind, name, exported)` of symbol `i` in `module`; `None` for trait members and trait
    /// impl items.
    fn shape(&self, i: usize, module: &str, vis: &str, kw: &'static str) -> Option<Shape> {
        let s = &self.g.symbols[i];
        Some(match owner(&s.scip, &self.traits) {
            Owner::Skip => return None,
            Owner::Free => (
                kw,
                s.name.clone(),
                vis == "pub" && self.reach.item(i, module),
            ),
            Owner::Inherent(ty) => (
                if kw == "fn" { "method" } else { kw },
                format!("{ty}::{}", s.name),
                vis == "pub" && self.reach.assoc(module, &ty),
            ),
        })
    }

    /// The item symbol `i` declares and the file it goes to; `None` for modules, private and
    /// test items, items outside the module tree, and trait members.
    fn item_of(&self, i: usize) -> Option<(&'a str, Item)> {
        let s = &self.g.symbols[i];
        if s.module_path.is_some() {
            return None;
        }
        let (vis, kw) = parse_sig(&s.signature)?;
        let path = self.g.files[s.file].path.as_str();
        if self.is_test(path, s) {
            return None;
        }
        let module = s.module.and_then(|m| self.ids.get(&m))?;
        let (kind, name, exported) = self.shape(i, module, vis, kw)?;
        let item = Item {
            symbol: symbol_name(&s.scip, &s.name),
            module: module.clone(),
            kind: kind.into(),
            name,
            signature: one_line(&s.signature),
            doc: first_paragraph(&s.doc),
            line: s.range.0 + 1,
            vis: vis.into(),
            visible_in: self.reach.visible_in(i, module, vis),
            exported,
        };
        Some((path, item))
    }
}

/// `(kind, name, exported)` of an item.
type Shape = (&'static str, String, bool);

/// Fills `items` of every compiled file.
pub fn add_items(
    files: &mut BTreeMap<String, FileData>,
    g: &Graph,
    ids: &ModuleIds,
    crate_ids: &[String],
    analyses: &BTreeMap<String, Analysis>,
    test_set: &HashSet<String>,
) {
    let items = Items::new(g, ids, crate_ids, analyses, test_set);
    for i in 0..g.symbols.len() {
        let Some((path, item)) = items.item_of(i) else {
            continue;
        };
        if let Some(fd) = files.get_mut(path) {
            fd.items.push(item);
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
