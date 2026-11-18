//! Brick interfaces: declared items, their uses, `interface/unused`, and the sites behind an
//! edge.
//!
//! The interface of a leaf brick is the set of its items that code outside the leaf can name:
//! the crate's public API, or items visible to a module subtree that reaches another brick (the
//! crate-internal interface between folders). The interface of a composite is the union of its
//! exported children, recursively. Everything is computed at `check` and `view` time from the
//! cache's items and per-file edges; this brick reads the config exports but never changes
//! anything.
//!
//! # Usage
//!
//! ```no_run
//! use brickie_check::{check, interface};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let l = check::load(Path::new("brickie.toml"))?;
//! let path = interface::parse_path(&l.cfg, "core/model")?;       // ["core", "model"]
//! let iface = interface::interface(&l.cache, &l.cfg, &l.resolved, &path);
//! for item in &iface.items {
//!     println!("{} {} used {}x", item.kind, item.name, item.use_count());
//! }
//! println!("{} unused", iface.unused());
//!
//! // The code behind the edge web -> core: every reference site, grouped by used item.
//! let web = vec!["web".to_string()];
//! let core = vec!["core".to_string()];
//! for it in interface::edge_uses(&l.cache, &l.resolved, &web, &core) {
//!     println!("{} ({} sites)", it.name, it.sites.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Key items
//!
//! - [`interface`] and [`Interface`]: the declared items of one brick with their uses
//!   ([`IfaceItem`], [`UseGroup`], [`Site`]).
//! - [`unused`]: one `interface/unused` finding per declared item no code outside the brick and
//!   no test uses.
//! - [`edge_uses`] and [`EdgeItem`]: the reference sites behind an edge between two bricks, any
//!   kind and scope, allowed or not.
//! - [`parse_path`]: `root/web/api` or `web/api` into brick names.
//!
//! # Notes
//!
//! - A use is a reference from outside the brick, or from test code anywhere. `pub use`
//!   re-exports are routes, not uses. A doctest naming a public API item is a test use (a
//!   word match: rust-analyzer does not index doc comments).
//! - Uses are grouped by the using brick at the interface's level; modules outside the tree are
//!   `(unmapped)`.
//! - Items are sorted by use count, descending, unused last.

mod declared;
mod doctest;
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
    /// How many declared items have no use.
    pub fn unused(&self) -> usize {
        self.items.iter().filter(|i| i.unused()).count()
    }
}

/// One declared item of a brick's interface, with where it is defined and who uses it.
#[derive(Debug, Clone, Serialize)]
pub struct IfaceItem {
    /// The leaf brick that defines the item (`root/web/api`).
    pub leaf: String,
    /// The defining module's id.
    pub module: String,
    pub symbol: String,
    /// `fn`, `struct`, `enum`, ... as in `brickie_facts::Item::kind`.
    pub kind: String,
    pub name: String,
    /// The one-line signature as rust-analyzer renders it.
    pub signature: String,
    /// The whole doc comment (markdown).
    pub doc: String,
    /// The defining file, relative to the workspace root.
    pub file: String,
    pub line: u32,
    /// Uses grouped by the using brick at the interface's level.
    pub uses: Vec<UseGroup>,
}

impl IfaceItem {
    /// The number of reference sites over all using bricks.
    pub fn use_count(&self) -> usize {
        self.uses.iter().map(|g| g.sites.len()).sum()
    }

    /// No code outside the brick and no test uses the item.
    pub fn unused(&self) -> bool {
        self.uses.is_empty()
    }

    /// The first paragraph of the doc comment, on one line.
    pub fn summary(&self) -> String {
        brickie_facts::doc_summary(&self.doc)
    }
}

/// The sites with which one brick uses an item.
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
    /// The file of the site, relative to the workspace root.
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    /// `prod` or `test`.
    pub scope: String,
    /// `uses`, `implements`, `reexports` or `doctest`.
    pub kind: String,
    /// The path as written at the site.
    pub written: String,
    /// The referenced symbol (an item, or a field / method / variant of one).
    pub symbol: String,
}

/// Parses `root/web/api` (or `web/api`) into brick names. Errors when no such brick exists.
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

/// The declared interface of the brick at `path` (brick names from the root, see
/// [`parse_path`]), with uses.
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
