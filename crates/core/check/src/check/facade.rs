//! The facade over the internal `rules` brick for the CLI and the webview.

use crate::config::{Config, Link};
use crate::rules;
use std::path::{Path, PathBuf};

pub use brickie_model::codes::{DEPS_CYCLE, DEPS_DENY, DEPS_PRIVATE};

/// Every allow rule between two named bricks that no measured edge backs, with its state.
/// The design todo list and the viewer read this one list.
pub fn unused_links(l: &super::Loaded) -> Vec<Link> {
    rules::unused_links(&l.cache, &l.cfg, &l.resolved)
}

/// Every rule between named bricks of `cfg`, allow or deny (no code read).
pub fn rule_links(cfg: &Config) -> Vec<Link> {
    rules::rule_links(cfg)
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
