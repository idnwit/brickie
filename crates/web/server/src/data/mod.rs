//! Builds the webview's data (`brickie_web_api::PageData`) from the loaded config and cache. The
//! diagnostics are the very values `brickie check` prints, converted field by field.

pub mod design;
mod diagnostics;
mod functions;
pub mod graph;
mod interface;
mod time;

pub use interface::{edge_uses, interface};
pub use time::utc;

use brickie_check::check::Loaded;
use brickie_web_api::*;
use diagnostics::{Lines, diag_view};
use std::path::Path;

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
