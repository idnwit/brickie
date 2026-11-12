//! Uses of an item: reference edges into its module whose symbol is the item (or a member of
//! it), from outside the brick or from test code anywhere; grouped by the using brick at the
//! brick's level. `pub use` re-exports are routes, not uses.

use super::{Site, UseGroup, covers, key};
use crate::config::{Resolved, level_name};
use brickie_facts::{Cache, FileEdge, Item};
use std::collections::{BTreeMap, HashMap};

pub(super) struct Ref<'a> {
    pub key: String,
    pub file: &'a str,
    pub edge: &'a FileEdge,
    /// Leaf path of the using module, `None` when unmapped.
    pub from: Option<&'a [String]>,
}

impl Ref<'_> {
    pub fn site(&self) -> Site {
        Site {
            file: self.file.to_string(),
            line: self.edge.line,
            col: self.edge.col,
            end_col: self.edge.end_col,
            scope: self.edge.scope.clone(),
            kind: self.edge.kind.clone(),
            written: self.edge.written.clone(),
            symbol: self.edge.symbol.clone(),
        }
    }
}

/// Reference edges (not re-exports) by target module.
pub(super) struct Index<'a> {
    pub by_to: HashMap<&'a str, Vec<Ref<'a>>>,
}

impl<'a> Index<'a> {
    pub fn new(cache: &'a Cache, res: &'a Resolved) -> Self {
        let mut by_to: HashMap<&str, Vec<Ref>> = HashMap::new();
        for (file, fd) in &cache.files {
            for e in fd.edges.iter().filter(|e| e.kind != "reexports") {
                by_to.entry(e.to.as_str()).or_default().push(Ref {
                    key: key(&e.symbol),
                    file,
                    edge: e,
                    from: res.path(&e.from),
                });
            }
        }
        Index { by_to }
    }

    /// Uses of `item` as part of the interface of the brick at `brick`.
    pub fn uses(&self, item: &Item, brick: &[String]) -> Vec<UseGroup> {
        let k = key(&item.symbol);
        let mut groups: BTreeMap<String, Vec<Site>> = BTreeMap::new();
        for r in self.by_to.get(item.module.as_str()).into_iter().flatten() {
            if !covers(&k, &r.key) {
                continue;
            }
            let inside = r.from.is_some_and(|p| p.starts_with(brick));
            if inside && r.edge.scope != "test" {
                continue;
            }
            let label = match r.from {
                None => "(unmapped)".to_string(),
                Some(_) if inside => format!("{} (tests)", level_name(brick)),
                Some(p) => level_name(&p[..brick.len().min(p.len())]),
            };
            groups.entry(label).or_default().push(r.site());
        }
        groups
            .into_iter()
            .map(|(brick, mut sites)| {
                sites.sort();
                sites.dedup();
                UseGroup { brick, sites }
            })
            .collect()
    }
}
