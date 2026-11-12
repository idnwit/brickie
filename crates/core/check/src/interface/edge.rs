//! The code behind an edge A -> B: every reference site from modules under A into modules under
//! B (any kind and scope, allowed or not), grouped by the used item of B.

use super::{Site, covers, key};
use crate::config::Resolved;
use brickie_facts::{Cache, Item};
use serde::Serialize;
use std::collections::BTreeMap;

/// One used item of the target brick and the sites that use it.
#[derive(Debug, Clone, Serialize)]
pub struct EdgeItem {
    pub module: String,
    /// The item's symbol, or the referenced symbol when it is no recorded item (private).
    pub symbol: String,
    pub name: String,
    /// `""` when no recorded item covers the symbol.
    pub kind: String,
    pub signature: String,
    /// Definition, when known.
    pub file: Option<String>,
    pub line: Option<u32>,
    pub sites: Vec<Site>,
}

/// A symbol key without its module path (`rules::deps::Dep::x` in `c::rules::deps` -> `Dep::x`).
fn local_name(module: &str, k: &str) -> String {
    match module.split_once("::") {
        Some((_, rel)) => k.strip_prefix(&format!("{rel}::")).unwrap_or(k).to_string(),
        None => k.to_string(),
    }
}

/// Sites of every edge from modules under `from` into modules under `to`, by used item;
/// items with the most sites first.
pub fn edge_uses(cache: &Cache, res: &Resolved, from: &[String], to: &[String]) -> Vec<EdgeItem> {
    let under = |m: &str, p: &[String]| res.path(m).is_some_and(|l| l.starts_with(p));
    // Recorded items per module: (key, item, file).
    let mut items: BTreeMap<&str, Vec<(String, &Item, &str)>> = BTreeMap::new();
    for (file, fd) in &cache.files {
        for i in &fd.items {
            items
                .entry(i.module.as_str())
                .or_default()
                .push((key(&i.symbol), i, file));
        }
    }
    let mut out: BTreeMap<(String, String), EdgeItem> = BTreeMap::new();
    for (file, fd) in &cache.files {
        for e in &fd.edges {
            if !under(&e.from, from) || !under(&e.to, to) {
                continue;
            }
            let k = key(&e.symbol);
            // The most specific recorded item that covers the symbol.
            let owner = items
                .get(e.to.as_str())
                .into_iter()
                .flatten()
                .filter(|(ik, _, _)| covers(ik, &k))
                .max_by_key(|(ik, _, _)| ik.len());
            let entry = out
                .entry((e.to.clone(), owner.map_or(k.clone(), |o| o.0.clone())))
                .or_insert_with(|| match owner {
                    Some((_, i, f)) => EdgeItem {
                        module: i.module.clone(),
                        symbol: i.symbol.clone(),
                        name: i.name.clone(),
                        kind: i.kind.clone(),
                        signature: i.signature.clone(),
                        file: Some(f.to_string()),
                        line: Some(i.line),
                        sites: Vec::new(),
                    },
                    None => EdgeItem {
                        module: e.to.clone(),
                        symbol: e.symbol.clone(),
                        name: local_name(&e.to, &k),
                        kind: String::new(),
                        signature: String::new(),
                        file: None,
                        line: None,
                        sites: Vec::new(),
                    },
                });
            entry.sites.push(Site {
                file: file.clone(),
                line: e.line,
                col: e.col,
                end_col: e.end_col,
                scope: e.scope.clone(),
                kind: e.kind.clone(),
                written: e.written.clone(),
                symbol: e.symbol.clone(),
            });
        }
    }
    let mut v: Vec<EdgeItem> = out.into_values().collect();
    for i in &mut v {
        i.sites.sort();
    }
    v.sort_by(|a, b| b.sites.len().cmp(&a.sites.len()).then(a.name.cmp(&b.name)));
    v
}
