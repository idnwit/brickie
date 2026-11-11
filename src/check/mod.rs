//! Config, cache and review combined into the diagnostics both `check` and `view` show.

use crate::cache::{Cache, SCHEMA_VERSION};
use crate::config::{self, Config};
use crate::diag::{self, Diagnostic};
use crate::{review, rules};
use anyhow::{Context, Result, bail};
use std::path::Path;

/// Config, cache and the diagnostics derived from them (review decisions applied, sorted).
/// Shared by `check` and `view`, so both show the same values.
pub struct Loaded {
    pub cfg: Config,
    pub cache: Cache,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn load(config: &Path) -> Result<Loaded> {
    let cfg = config::load(config)?;
    let cache = read_cache(config)?;
    let review = review::read(config)?;
    let root = Path::new(&cache.workspace_root);
    let mut found = rules::evaluate(&cache, &cfg, root);
    found.extend(rules::evaluate_stale(&cache, &cfg));
    let mut diagnostics: Vec<Diagnostic> =
        found.iter().map(|f| diag::diagnostic(f.as_ref())).collect();
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
        diagnostics,
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
