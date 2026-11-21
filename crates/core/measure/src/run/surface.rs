//! The surface facts of an item: text that names the types and traits a caller can reach
//! through the item without writing their names. See `Item::surface`.

use super::complexity::Analysis;
use super::items::{one_line, parse_sig};
use crate::scip::{Graph, SymbolRow};
use ::scip::symbol::{format_symbol, parse_symbol};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Members and implemented traits by owning item.
pub struct Surface {
    /// Owning item symbol -> member signatures.
    members: HashMap<String, Vec<String>>,
    /// Type name -> traits it implements.
    impls: HashMap<String, BTreeSet<String>>,
}

/// Item keyword of `scip`, for structs, enums, unions and traits only.
fn owner_kind(g: &Graph) -> HashMap<&str, &'static str> {
    g.symbols
        .iter()
        .filter_map(|s| {
            let (_, kw) = parse_sig(&s.signature)?;
            matches!(kw, "struct" | "enum" | "union" | "trait").then_some((s.scip.as_str(), kw))
        })
        .collect()
}

/// The nearest struct, enum, union or trait above `s` and whether `s` is its direct member.
fn owner(
    s: &SymbolRow,
    kinds: &HashMap<&str, &'static str>,
) -> Option<(String, &'static str, bool)> {
    let sym = parse_symbol(&s.scip).ok()?;
    let n = sym.descriptors.len();
    (1..n).rev().find_map(|k| {
        let mut parent = sym.clone();
        parent.descriptors.truncate(k);
        let parent = format_symbol(parent);
        let kw = *kinds.get(parent.as_str())?;
        Some((parent, kw, k == n - 1))
    })
}

/// Whether a member with signature `sig` is surface of an owner of kind `kw`.
fn shown(kw: &str, sig: &str, direct: bool) -> bool {
    match kw {
        "enum" => sig.starts_with("pub ") || direct,
        "trait" => direct,
        _ => sig.starts_with("pub "),
    }
}

impl Surface {
    pub fn new(g: &Graph, analyses: &BTreeMap<String, Analysis>) -> Self {
        let kinds = owner_kind(g);
        let mut members: HashMap<String, Vec<String>> = HashMap::new();
        let candidates = g.symbols.iter().filter(|s| {
            s.module_path.is_none() && !s.signature.is_empty() && parse_sig(&s.signature).is_none()
        });
        for s in candidates {
            let sig = one_line(&s.signature);
            if let Some((parent, ..)) = owner(s, &kinds).filter(|o| shown(o.1, &sig, o.2)) {
                members.entry(parent).or_default().push(sig);
            }
        }
        let mut impls: HashMap<String, BTreeSet<String>> = HashMap::new();
        for a in analyses.values() {
            for (ty, tr) in &a.impls {
                impls.entry(ty.clone()).or_default().insert(tr.clone());
            }
        }
        Surface { members, impls }
    }

    /// Field and variant signatures of a struct, enum or union, member signatures of a trait,
    /// and for a type the traits it implements (by name).
    pub fn of(&self, scip: &str, kind: &str, name: &str) -> Vec<String> {
        let mut out = self.members.get(scip).cloned().unwrap_or_default();
        if matches!(kind, "struct" | "enum" | "union") {
            out.extend(self.impls.get(name).into_iter().flatten().cloned());
        }
        out.sort();
        out.dedup();
        out
    }
}
