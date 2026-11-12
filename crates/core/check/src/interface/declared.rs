//! The declared interface: the exports chain down to leaves, and each leaf's public items.

use crate::config::{Config, Resolved};
use brickie_facts::{Cache, Item};
use std::collections::{BTreeMap, BTreeSet};

/// `id` is `scope` or below it.
fn in_subtree(id: &str, scope: &str) -> bool {
    id == scope || id.strip_prefix(scope).is_some_and(|r| r.starts_with("::"))
}

/// A declared item with the leaf that defines it.
pub(super) struct Declared<'a> {
    pub leaf: Vec<String>,
    pub item: &'a Item,
    pub file: &'a str,
}

/// The leaves whose items make up the interface of the brick at `path`: the brick itself for
/// a leaf; for a composite, the leaves of its exported children (recursively).
pub(super) fn leaves_of(cfg: &Config, path: &[String]) -> Vec<Vec<String>> {
    let Some(b) = cfg.brick_at(path) else {
        return Vec::new();
    };
    let Some(sub) = &b.sub else {
        return vec![path.to_vec()];
    };
    let names: BTreeSet<&str> = sub.exports.iter().map(|e| e.brick.as_str()).collect();
    names
        .into_iter()
        .flat_map(|n| {
            let mut p = path.to_vec();
            p.push(n.to_string());
            leaves_of(cfg, &p)
        })
        .collect()
}

/// Declared items of the brick at `path`: items of its leaves that code outside the leaf can
/// name. That is the crate's public API (`exported`), or a scope (`visible_in`) whose module
/// subtree reaches a module of another brick (the crate-internal interface between folders).
pub(super) fn declared<'a>(
    cache: &'a Cache,
    cfg: &Config,
    res: &Resolved,
    path: &[String],
) -> Vec<Declared<'a>> {
    let leaves: BTreeSet<Vec<String>> = leaves_of(cfg, path).into_iter().collect();
    // Leaf paths of the modules in each scope's subtree, memoised per (scope, leaf).
    let mut outside: BTreeMap<(&str, &[String]), bool> = BTreeMap::new();
    let mut out = Vec::new();
    for (file, fd) in &cache.files {
        for item in &fd.items {
            let Some(leaf) = res.path(&item.module) else {
                continue;
            };
            if !leaves.contains(leaf) {
                continue;
            }
            let reaches_out = *outside
                .entry((item.visible_in.as_str(), leaf))
                .or_insert_with(|| {
                    cache.modules.iter().any(|m| {
                        in_subtree(&m.id, &item.visible_in) && res.path(&m.id) != Some(leaf)
                    })
                });
            if item.exported || reaches_out {
                out.push(Declared {
                    leaf: leaf.to_vec(),
                    item,
                    file,
                });
            }
        }
    }
    out
}
