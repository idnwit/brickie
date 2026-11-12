//! Brick interfaces: what a brick declares public, who uses each item, and which items nobody
//! outside the brick (and no test) uses (`interface/unused`). Also the reference sites behind
//! an edge between two bricks.
//!
//! Computed from the cache's items and per-file edges at `check` / `view` time.

mod declared;
mod edge;
mod unused;
mod uses;

#[cfg(test)]
mod tests;

use declared::declared;
pub use edge::{EdgeItem, edge_uses};
pub use unused::unused;

use crate::config::{Config, Resolved, level_name};
use anyhow::{Result, bail};
use brickie_facts::Cache;
use serde::Serialize;

/// The declared interface of one brick.
#[derive(Debug, Clone, Serialize)]
pub struct Interface {
    /// `root/web/api`.
    pub brick: String,
    /// Sorted by use count (descending), unused last; ties by name.
    pub items: Vec<IfaceItem>,
}

impl Interface {
    pub fn unused(&self) -> usize {
        self.items.iter().filter(|i| i.unused()).count()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IfaceItem {
    /// The leaf brick that defines the item (`root/web/api`).
    pub leaf: String,
    pub module: String,
    pub symbol: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub doc: String,
    pub file: String,
    pub line: u32,
    /// Uses grouped by the using brick at the interface's level.
    pub uses: Vec<UseGroup>,
}

impl IfaceItem {
    pub fn use_count(&self) -> usize {
        self.uses.iter().map(|g| g.sites.len()).sum()
    }

    pub fn unused(&self) -> bool {
        self.uses.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UseGroup {
    /// Using brick at the interface's level (`root/web/server`); the brick itself with
    /// ` (tests)` for its own tests; `(unmapped)` for modules outside the tree.
    pub brick: String,
    pub sites: Vec<Site>,
}

/// One reference site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Site {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    /// `prod` or `test`.
    pub scope: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    /// The path as written at the site.
    pub written: String,
    /// The referenced symbol (an item, or a field / method / variant of one).
    pub symbol: String,
}

/// Parses `root/web/api` (or `web/api`) into brick names.
pub fn parse_path(cfg: &Config, path: &str) -> Result<Vec<String>> {
    let names: Vec<String> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let names = match names.split_first() {
        Some((h, rest)) if h == "root" => rest.to_vec(),
        _ => names,
    };
    if names.is_empty() || cfg.brick_at(&names).is_none() {
        bail!("no brick `{path}` (use a path such as root/web/api)");
    }
    Ok(names)
}

/// The declared interface of the brick at `path`, with uses.
pub fn interface(cache: &Cache, cfg: &Config, res: &Resolved, path: &[String]) -> Interface {
    let idx = uses::Index::new(cache, res);
    let mut items: Vec<IfaceItem> = declared(cache, cfg, res, path)
        .into_iter()
        .map(|d| {
            let uses = idx.uses(d.item, path);
            IfaceItem {
                leaf: level_name(&d.leaf),
                module: d.item.module.clone(),
                symbol: d.item.symbol.clone(),
                kind: d.item.kind.clone(),
                name: d.item.name.clone(),
                signature: d.item.signature.clone(),
                doc: d.item.doc.clone(),
                file: d.file.to_string(),
                line: d.item.line,
                uses,
            }
        })
        .collect();
    items.sort_by(|a, b| {
        (
            a.unused(),
            std::cmp::Reverse(a.use_count()),
            &a.name,
            &a.file,
        )
            .cmp(&(
                b.unused(),
                std::cmp::Reverse(b.use_count()),
                &b.name,
                &b.file,
            ))
    });
    Interface {
        brick: level_name(path),
        items,
    }
}

/// Item symbol key: `impl` segments and generic arguments dropped
/// (`a::impl::Ctx<'_>::txt` -> `a::Ctx::txt`).
fn key(symbol: &str) -> String {
    symbol
        .split("::")
        .filter(|s| *s != "impl")
        .map(|s| s.split('<').next().unwrap_or(s))
        .collect::<Vec<_>>()
        .join("::")
}

/// Whether a reference to `used` (a key) is a use of the item with key `item`: the item
/// itself or one of its fields, methods, variants or associated items.
fn covers(item: &str, used: &str) -> bool {
    used == item
        || used
            .strip_prefix(item)
            .is_some_and(|rest| rest.starts_with("::"))
}
