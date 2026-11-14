//! Config, cache and review combined into the diagnostics both `check` and `view` show.

use crate::config::{self, Config, Mode, Resolved};
use crate::diag::{self, Diagnostic};
use crate::{interface, review, rules};
use anyhow::{Context, Result, bail};
use brickie_facts::{Cache, SCHEMA_VERSION};

mod facade;
mod scope;
pub use facade::*;
pub use scope::{Counts, Filter, Filtered};
use std::collections::BTreeMap;
use std::path::Path;

/// Config, cache and the diagnostics derived from them (review decisions applied, sorted).
/// Shared by `check` and `view`, so both show the same values.
pub struct Loaded {
    /// The brick tree (root level).
    pub cfg: Config,
    pub cache: Cache,
    /// Module id -> leaf path in the tree.
    pub resolved: Resolved,
    pub diagnostics: Vec<Diagnostic>,
    /// Diagnostic id -> owner brick paths (`root/web/api`), for scoped checks.
    pub owners: BTreeMap<String, Vec<String>>,
}

pub fn load(config: &Path) -> Result<Loaded> {
    load_from(&config::Disk, config)
}

/// `load` with the config tree read from `src` (a change's preview); the cache and the review
/// file are read from disk.
pub fn load_from(src: &dyn config::FileSource, config: &Path) -> Result<Loaded> {
    load_mode(src, config, Mode::Strict)
}

/// `load_from` for a design in progress: partition errors (`config/unassigned`, ...) do not
/// abort; they are in `resolved.pending`. Parse and validation errors still do.
pub fn load_draft_from(src: &dyn config::FileSource, config: &Path) -> Result<Loaded> {
    load_mode(src, config, Mode::Draft)
}

fn load_mode(src: &dyn config::FileSource, config: &Path, mode: Mode) -> Result<Loaded> {
    let cfg = config::load_from(src, config)?;
    let cache = read_cache(config)?;
    let review = review::read(config)?;
    let root = Path::new(&cache.workspace_root);
    let ws = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let resolved = config::resolve(&cfg, &cache.crates, &cache.modules, &ws, mode)?;
    let mut found = rules::evaluate(&cache, &cfg, &resolved, root);
    found.extend(rules::evaluate_stale(&cache));
    found.extend(interface::unused(&cache, &cfg, &resolved));
    let mut diagnostics: Vec<Diagnostic> =
        found.iter().map(|f| diag::diagnostic(f.as_ref())).collect();
    let owners = (found.iter())
        .map(|f| (diag::finding_id(&f.code(), &f.subject()), f.owners()))
        .collect();
    for d in &mut diagnostics {
        if let Some(a) = review.accept.iter().find(|a| a.id == d.id) {
            d.accepted = true;
            d.accepted_reason = Some(a.reason.clone());
        }
    }
    diagnostics.sort_by(|a, b| (a.level, &a.code, &a.message).cmp(&(b.level, &b.code, &b.message)));
    Ok(Loaded {
        cfg,
        cache,
        resolved,
        diagnostics,
        owners,
    })
}

fn read_cache(config: &Path) -> Result<Cache> {
    let cache_path = review::config_dir(config).join(".brickie/cache.json");
    let bytes = std::fs::read(&cache_path).with_context(|| {
        format!(
            "read {} (run `brickie measure` first)",
            cache_path.display()
        )
    })?;
    let cache: Cache = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {} (run `brickie measure`)", cache_path.display()))?;
    if cache.schema_version != SCHEMA_VERSION {
        bail!(
            "{} has schema {}, expected {SCHEMA_VERSION}; run `brickie measure`",
            cache_path.display(),
            cache.schema_version
        );
    }
    Ok(cache)
}
