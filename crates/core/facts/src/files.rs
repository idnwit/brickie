//! Walking the source tree: which `.rs` files and crate directories a workspace has.

use super::Crate;

/// Every `.rs` path (relative to `root`) under `dir`, skipping `target` directories.
pub fn walk_rs(
    dir: &std::path::Path,
    root: &std::path::Path,
    out: &mut std::collections::BTreeSet<String>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if e.file_name() != "target" {
                walk_rs(&p, root, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs")
            && let Ok(rel) = p.strip_prefix(root)
        {
            out.insert(rel.to_string_lossy().into_owned());
        }
    }
}

/// Directories holding crate roots (relative to the workspace root; "" for the root itself).
pub fn crate_dirs(crates: &[Crate]) -> std::collections::BTreeSet<String> {
    crates
        .iter()
        .map(|c| c.root.rsplit_once('/').map_or("", |(d, _)| d).to_string())
        .collect()
}
