//! Load config and cache into diagnostics: the facade for the CLI and the webview.
//!
//! [`load`] reads the config tree and the measure cache, resolves every module to a brick,
//! runs every rule and returns one [`Loaded`] value. `brickie check` prints it and the viewer
//! shows it, so both see the same findings. The internal `rules` brick stays hidden behind this
//! module.
//!
//! # Usage
//!
//! ```no_run
//! use brickie_check::check::{self, Filter};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = Path::new("brickie.toml");
//! if check::needs_measure(config) {
//!     // run `brickie measure` first
//! }
//! let loaded = check::load(config)?;
//! // Show only findings owned by bricks under `web/api`; the whole workspace is still evaluated.
//! let filter = Filter::new(&loaded.cfg, &["web/api".to_string()])?;
//! let shown = filter.apply(&loaded);
//! print!("{}", shown.footer(&filter));
//! for d in &shown.shown {
//!     println!("{} {}", d.code, d.message);
//! }
//! for line in loaded.todo.lines() {
//!     println!("planned: {line}");                  // never a finding, never an exit code
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Key items
//!
//! - [`load`] and [`load_from`]: config tree plus cache into [`Loaded`]; `load_from` reads the
//!   config from a [`crate::config::FileSource`] (the design draft or a preview).
//! - [`load_design`]: the config tree alone, with no cache and no findings.
//! - [`needs_measure`]: whether the cache is missing, unreadable or stale.
//! - [`Filter`] and [`Filtered`]: `check --brick` scoping.
//! - [`Todo`]: planned bricks and links the code lacks yet.
//! - [`marked_links`], [`design_links`], [`complexity_limits`], [`dep_subject`],
//!   [`denied_text`]: helpers the viewer and design editor share with the rules.
//!
//! # Notes
//!
//! - Errors: a missing, unparsable or other-schema cache is an `Err` that tells to run
//!   `brickie measure`; config problems found while loading are findings, not errors.
//! - A cache measured in another directory yields one `stale/measure` finding and nothing else
//!   about code.
//! - `check` never writes and never measures.

use crate::config::{self, Config, Resolved};
use crate::diag::{self, Diagnostic};
use crate::{interface, rules};
use anyhow::{Context, Result, bail};
use brickie_facts::{Cache, SCHEMA_VERSION};

mod facade;
mod scope;
mod todo;
pub use facade::*;
pub use scope::{Filter, Filtered};
use std::collections::BTreeMap;
use std::path::Path;
pub use todo::Todo;

/// Config, cache and the diagnostics derived from them (sorted).
/// Shared by `check` and `view`, so both show the same values.
pub struct Loaded {
    /// The brick tree (root level).
    pub cfg: Config,
    /// The measure cache the findings were derived from.
    pub cache: Cache,
    /// Module id -> leaf path in the tree.
    pub resolved: Resolved,
    /// Every finding, sorted by code then message.
    pub diagnostics: Vec<Diagnostic>,
    /// Diagnostic id -> owner brick paths (`root/web/api`), for scoped checks.
    pub owners: BTreeMap<String, Vec<String>>,
    /// Config issues (`config/missing` and the partition errors), also in `diagnostics`.
    pub issues: Vec<config::Issue>,
    /// Planned work: never a finding, never counted in an exit code.
    pub todo: Todo,
}

/// Load the config at `config` and the cache next to it (`.brickie/cache.json`) and evaluate
/// every rule. Errors when the cache is missing, unparsable or of another schema version, or the
/// config cannot be read or parsed.
pub fn load(config: &Path) -> Result<Loaded> {
    load_from(&config::Disk, config)
}

/// `load` with the config tree read from `src` (the design draft); the cache is read from
/// disk. Config issues and denied links are findings; planned work is `todo`.
pub fn load_from(src: &dyn config::FileSource, config: &Path) -> Result<Loaded> {
    let cfg = config::load_from(src, config)?;
    let cache = read_cache(config)?;
    let root = Path::new(&cache.workspace_root);
    let ws = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let (resolved, partition) = config::resolve(&cfg, &cache.crates, &cache.modules, &ws);
    let mut issues = cfg.missing.clone();
    issues.extend(partition);
    let mut found = rules::evaluate(&cache, &cfg, &resolved, root);
    for i in &issues {
        found.push(Box::new(rules::IssueFinding(i.clone())));
    }
    for s in crate::rules::design_links(&cfg) {
        if let Some(why) = s.denied.clone() {
            found.push(Box::new(rules::LinkDenied(s, why)));
        }
    }
    // A cache from another directory describes another tree: report that alone.
    let here = config_dir(config);
    let here = here.canonicalize().unwrap_or(here);
    match rules::evaluate_stale_location(&cache, &here) {
        Some(moved) => found.push(moved),
        None => {
            found.extend(rules::evaluate_stale(&cache));
            found.extend(rules::evaluate_stale_measure(&cache, &cfg.measure));
        }
    }
    found.extend(interface::unused(&cache, &cfg, &resolved));
    let mut diagnostics: Vec<Diagnostic> =
        found.iter().map(|f| diag::diagnostic(f.as_ref())).collect();
    let owners = (found.iter())
        .map(|f| (diag::finding_id(&f.code(), &f.subject()), f.owners()))
        .collect();
    diagnostics.sort_by(|a, b| (&a.code, &a.message).cmp(&(&b.code, &b.message)));
    let todo = todo::todo(&cache, &cfg, &resolved);
    Ok(Loaded {
        cfg,
        cache,
        resolved,
        diagnostics,
        owners,
        issues,
        todo,
    })
}

/// The config tree read from `src` alone: no cache, no module, no finding. Design mode shows
/// this; code is compared with the design only by the publish gate.
pub fn load_design(src: &dyn config::FileSource, config: &Path) -> Result<Loaded> {
    Ok(Loaded {
        cfg: config::load_from(src, config)?,
        cache: Cache::default(),
        resolved: Resolved::default(),
        diagnostics: Vec::new(),
        owners: BTreeMap::new(),
        issues: Vec::new(),
        todo: Todo::default(),
    })
}

/// Whether `brickie measure` would help: the cache is missing, unreadable or of another schema,
/// its files changed (judged from the cache alone, so a config error that old facts cause, such
/// as `config/unassigned` for a crate that is gone, does not block the re-measure), or the
/// loaded result holds a `stale/*` finding.
pub fn needs_measure(config: &Path) -> bool {
    let Ok(cache) = read_cache(config) else {
        return true;
    };
    let here = config_dir(config);
    let here = here.canonicalize().unwrap_or(here);
    if rules::evaluate_stale_location(&cache, &here).is_some()
        || !rules::evaluate_stale(&cache).is_empty()
    {
        return true;
    }
    load(config).is_ok_and(|l| (l.diagnostics.iter()).any(|d| d.code.starts_with("stale/")))
}

fn read_cache(config: &Path) -> Result<Cache> {
    let cache_path = config_dir(config).join(".brickie/cache.json");
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

#[cfg(test)]
mod tests {
    use super::{Filter, load};
    use crate::config::tests::{cache, edge, fixture, func};

    #[test]
    fn group_scope_filter_and_todo_groups() {
        let root = r#"
[[group]]
name = "svc"
tags = ["domain"]

[[group]]
name = "empty_g"

[[brick]]
name = "auth"
group = "svc"
members = ["web-api"]

[[brick]]
name = "billing"
group = "svc"
members = ["web-ui"]

[[brick]]
name = "bus"
members = ["core"]

[[brick]]
name = "unused"
members = ["web-shared"]

[[complexity]]
bricks = "svc"
cognitive = 5
"#;
        let (dir, cfg) = fixture(root, "");
        let ws = dir.path().canonicalize().unwrap();
        let c = cache(
            &ws,
            &[
                (
                    "web-api",
                    vec![
                        edge("web-api", "core", "prod"),
                        edge("web-api", "web-ui", "prod"),
                    ],
                    vec![func("auth_fn", "web-api", 12)],
                ),
                ("core", vec![edge("core", "web-ui", "prod")], vec![]),
            ],
        );
        let cache_path = dir.path().join(".brickie/cache.json");
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, serde_json::to_string(&c).unwrap()).unwrap();

        let loaded = load(&dir.path().join("brickie.toml")).unwrap();
        let by_group = Filter::new(&cfg, &["svc".into()]).unwrap().apply(&loaded);
        assert!(by_group.shown.iter().any(|d| d.code == "deps/cycle"));
        assert!(
            by_group
                .shown
                .iter()
                .any(|d| d.code == "complexity/cognitive")
        );

        let by_member = Filter::new(&cfg, &["auth".into()]).unwrap().apply(&loaded);
        assert!(by_member.shown.iter().any(|d| d.code == "deps/cycle"));

        assert_eq!(loaded.todo.groups, vec![vec!["empty_g".to_string()]]);
        assert!(
            loaded
                .todo
                .lines()
                .contains(&"planned group root/empty_g has no bricks yet".to_string())
        );
    }
}
