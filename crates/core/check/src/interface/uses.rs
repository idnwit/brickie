//! Uses of an item: reference edges into its module whose symbol is the item (or a member of
//! it), from outside the brick or from test code anywhere; grouped by the using brick at the
//! brick's level. `pub use` re-exports are routes, not uses. Doctests of the item's crate that
//! name it are test uses inside the brick ([`super::doctest`]).

use super::doctest::Doctests;
use super::surface::{self, Via};
use super::{Site, UseGroup, covers, key};
use crate::config::{Resolved, level_name};
use brickie_facts::{Cache, FileEdge, Item};
use std::collections::{BTreeMap, HashMap, HashSet};

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
    doctests: Doctests,
    /// Crate ids of packages that may be published.
    published: HashSet<&'a str>,
    /// Module id -> crate id.
    crate_of: HashMap<&'a str, &'a str>,
    res: &'a Resolved,
    /// Items named by the surface of a used item ([`surface`]).
    via: HashMap<surface::Id<'a>, Via<'a>>,
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
        let mut idx = Index {
            by_to,
            doctests: Doctests::new(cache),
            published: cache
                .crates
                .iter()
                .filter(|c| c.publish)
                .map(|c| c.id.as_str())
                .collect(),
            crate_of: cache
                .modules
                .iter()
                .map(|m| (m.id.as_str(), m.krate.as_str()))
                .collect(),
            res,
            via: HashMap::new(),
        };
        let via = surface::reach(cache, &idx.crate_of, |i| {
            idx.published(i)
                || res
                    .path(&i.module)
                    .is_some_and(|leaf| !idx.direct(i, leaf).is_empty())
        });
        idx.via = via;
        idx
    }

    /// The item is public API of a package that may be published: code outside the workspace,
    /// which brickie cannot see, may use it, so it is never reported unused.
    pub fn published(&self, item: &Item) -> bool {
        item.exported
            && self
                .crate_of
                .get(item.module.as_str())
                .is_some_and(|k| self.published.contains(k))
    }

    /// Uses of `item` as part of the interface of the brick at `brick`: its own uses, or when it
    /// has none, one group `<brick> (via surface)` with a `surface` site at the definition of
    /// the used item whose surface names it. The brick is the one of that item at the level of
    /// `brick`. Whether that item is used is judged at the level of its leaf brick.
    pub fn uses(&self, item: &Item, brick: &[String]) -> Vec<UseGroup> {
        let mut groups = self.direct(item, brick);
        if let (true, Some(v)) = (groups.is_empty(), self.via.get(&surface::id(item))) {
            let label = match self.res.path(&v.item.module) {
                Some(p) => level_name(&p[..brick.len().min(p.len())]),
                None => "(unmapped)".to_string(),
            };
            groups.push(UseGroup {
                brick: format!("{label} (via surface)"),
                sites: vec![Site {
                    file: v.file.to_string(),
                    line: v.item.line,
                    col: 1,
                    end_col: 1,
                    scope: "prod".into(),
                    kind: "surface".into(),
                    written: v.item.name.clone(),
                    symbol: v.item.symbol.clone(),
                }],
            });
        }
        groups
    }

    /// References and doctests that use `item`, from outside the brick at `brick` or from tests.
    fn direct(&self, item: &Item, brick: &[String]) -> Vec<UseGroup> {
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
        let doc = self.doctests.sites(item);
        if !doc.is_empty() {
            let label = format!("{} (tests)", level_name(brick));
            groups.entry(label).or_default().extend(doc);
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
