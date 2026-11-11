//! Builds the webview's data (`brickie_ui::PageData`) from the loaded config and cache. The
//! diagnostics are the very values `brickie check` prints, converted field by field.

mod diagnostics;
mod functions;
mod graph;
mod time;

pub use diagnostics::split_bytes;
pub use time::utc;

use crate::check::Loaded;
use brickie_ui::*;
use diagnostics::{Lines, diag_view};
use std::path::Path;

pub fn page(l: &Loaded, cache_file: &Path) -> PageData {
    let cache = &l.cache;
    let mut lines = Lines::new(Path::new(&cache.workspace_root));
    PageData {
        cache: CacheInfo {
            workspace: cache.workspace_root.clone(),
            commit: cache.git.as_ref().map(|g| g.commit.clone()),
            dirty: cache.git.as_ref().is_some_and(|g| g.dirty),
            measured: std::fs::metadata(cache_file)
                .and_then(|m| m.modified())
                .map_or_else(|_| "unknown".into(), utc),
        },
        diags: l
            .diagnostics
            .iter()
            .map(|d| diag_view(d, &mut lines))
            .collect(),
        graph: graph::graph(l),
        functions: functions::functions(l),
    }
}
