//! Builds the webview's data from the loaded config and cache.
//!
//! This brick maps what `brickie-check` and the cache already computed into the types of
//! [`brickie_web_api`]: the page ([`page`]), a brick's docs page ([`docs`]), a brick's interface
//! ([`interface()`]) and the sites behind an edge ([`edge_uses`]), plus the design editor's state
//! (in `design`) and the level graphs (in `graph`). It decides nothing about rules: diagnostics
//! are the very values `brickie check` prints, converted field by field, and whether an edge
//! is allowed comes from the check crate. It does no HTTP and no HTML.
//!
//! # Usage
//!
//! Load once per request with `brickie_change::working::load`, then convert.
//!
//! ```ignore
//! // Crate-private: shown for orientation only.
//! let (loaded, _) = brickie_change::working::load(&config)?;
//! let page = data::page(&loaded, &cache_file);        // PageData for `GET /`
//! let docs = data::docs(&loaded, "web/api")?;         // DocsPage for `GET /docs/web/api`
//! let iface = data::interface(&loaded, "root/web/api")?;
//! let sites = data::edge_uses(&loaded, "web/ui", "web/api", "uses")?;
//! ```
//!
//! # Key functions
//!
//! - [`page`]: levels, diagnostics, function rows and the cache header.
//! - [`docs`]: the brick tree, the root module's `//!` doc and the declared items with docs and
//!   a few real use sites per using brick.
//! - [`interface()`], [`edge_uses`]: the interface view and the use sites of one edge by kind.
//!
//! # Errors
//!
//! [`docs`], [`interface()`] and [`edge_uses`] fail on an unknown brick path; the routes turn that
//! into a 404 or 409. Nothing here writes files except the design layout helpers in `design`.

pub mod design;
mod diagnostics;
mod docs;
mod functions;
pub mod graph;
mod interface;
mod time;

pub use docs::docs;
pub use interface::{edge_uses, interface};

use brickie_check::check::Loaded;
use brickie_web_api::*;
use diagnostics::{Lines, diag_view};
use std::path::Path;
use time::utc;

pub fn page(l: &Loaded, cache_file: &Path) -> PageData {
    let cache = &l.cache;
    let mut lines = Lines::new(Path::new(&cache.workspace_root));
    let modified = std::fs::metadata(cache_file)
        .and_then(|m| m.modified())
        .ok();
    PageData {
        cache: CacheInfo {
            workspace: cache.workspace_root.clone(),
            commit: cache.git.as_ref().map(|g| g.commit.clone()),
            dirty: cache.git.as_ref().is_some_and(|g| g.dirty),
            measured: modified.map_or_else(|| "unknown".into(), utc),
            age_secs: modified.and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()),
            stale_files: l
                .diagnostics
                .iter()
                .filter(|d| d.code == brickie_model::codes::STALE_FILE)
                .count(),
        },
        diags: l
            .diagnostics
            .iter()
            .map(|d| diag_view(d, &mut lines))
            .collect(),
        levels: graph::levels(l),
        functions: functions::functions(l),
        design: None,
    }
}
