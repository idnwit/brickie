//! The facade over the internal `rules` brick for the CLI and the webview.

use crate::config::{Config, LinkState};
use crate::rules;
use std::path::{Path, PathBuf};

pub use brickie_model::codes::{DEPS_CYCLE, DEPS_PRIVATE, DEPS_UNLINKED};

/// The valid links no measured edge uses yet (information for `design check` and the viewer).
pub fn planned_links(l: &super::Loaded) -> Vec<LinkState> {
    rules::planned_links(&l.cache, &l.cfg, &l.resolved)
}

/// Every link of the design, with `planned` set (information for the viewer).
pub fn marked_links(l: &super::Loaded) -> Vec<LinkState> {
    rules::marked_links(&l.cache, &l.cfg, &l.resolved)
}

/// Every link of `cfg`, judged against its level's rules (no code read).
pub fn design_links(cfg: &Config) -> Vec<LinkState> {
    rules::design_links(cfg)
}

/// The subject of a dependency finding: `from->to:kind:scope` over module ids.
pub fn dep_subject(from: &str, to: &str, kind: &str, scope: &str) -> String {
    rules::dep_subject(from, to, kind, scope)
}

/// `(metric, value, limit)` for every metric of `func` that a complexity rule limits for the leaf
/// at `leaf`.
pub fn complexity_limits(
    cfg: &Config,
    leaf: &[String],
    func: &brickie_facts::Function,
) -> Vec<(&'static str, u64, u64)> {
    rules::complexity_limits(cfg, leaf, func)
}

/// The directory of the config file (`.` for a bare file name).
pub fn config_dir(config: &Path) -> PathBuf {
    config
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}
