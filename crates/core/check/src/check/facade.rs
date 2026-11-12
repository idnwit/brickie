//! The facade over the internal `rules` and `review` bricks for the CLI and the webview.

use crate::config::Config;
use crate::{review, rules};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub use brickie_model::codes::{DEPS_CYCLE, DEPS_DEFAULT, DEPS_DENY, DEPS_PRIVATE};

/// The subject of a dependency finding: `from->to:kind:scope` over module ids.
pub fn dep_subject(from: &str, to: &str, kind: &str, scope: &str) -> String {
    rules::dep_subject(from, to, kind, scope)
}

/// `(metric, value, limit)` for every metric of `func` that a complexity rule of the leaf at
/// `leaf` (or of an ancestor level) limits.
pub fn complexity_limits(
    cfg: &Config,
    leaf: &[String],
    func: &brickie_facts::Function,
) -> Vec<(&'static str, u64, u64)> {
    rules::complexity_limits(cfg, leaf, func)
}

/// The directory of the config file (`.` for a bare file name).
pub fn config_dir(config: &Path) -> PathBuf {
    review::config_dir(config)
}

/// Accept (`Some(reason)`) or unaccept (`None`) the finding `id` in `brickie.review.toml`.
pub fn set_review(config: &Path, id: &str, reason: Option<&str>) -> Result<()> {
    review::set_review(config, id, reason)
}
