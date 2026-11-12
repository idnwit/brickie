//! Planned bricks: bricks of the design no measured module maps to yet.

use brickie_check::check::Loaded;
use brickie_check::config::{Config, Resolved};
use std::collections::BTreeSet;
use std::path::Path;

/// The message reported instead of mass `planned` when the cache was measured elsewhere: its
/// workspace root is unrelated to `root`, or no measured module maps to any brick.
pub fn cache_mismatch(root: &Path, loaded: &Loaded) -> Option<String> {
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let cache_root = canon(Path::new(&loaded.cache.workspace_root));
    let root = canon(root);
    let related = root.starts_with(&cache_root) || cache_root.starts_with(&root);
    let unmatched = !loaded.cfg.bricks.is_empty() && loaded.resolved.paths.is_empty();
    (!related || unmatched)
        .then(|| "cache does not match this workspace; run brickie measure".to_string())
}

/// Paths (from the root) of the bricks with no measured module, in tree order.
pub fn planned(cfg: &Config, resolved: &Resolved) -> Vec<Vec<String>> {
    let mut built: BTreeSet<&[String]> = BTreeSet::new();
    for path in resolved.paths.values() {
        for n in 1..=path.len() {
            built.insert(&path[..n]);
        }
    }
    let mut out = Vec::new();
    walk(cfg, &built, &mut out);
    out
}

fn walk(level: &Config, built: &BTreeSet<&[String]>, out: &mut Vec<Vec<String>>) {
    for b in &level.bricks {
        let mut path = level.path.clone();
        path.push(b.name.clone());
        if !built.contains(path.as_slice()) {
            out.push(path);
        }
        if let Some(sub) = &b.sub {
            walk(sub, built, out);
        }
    }
}
