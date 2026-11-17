//! The cache-mismatch guard for the planned list (the list itself is `Loaded::todo`).

use brickie_check::check::Loaded;
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
