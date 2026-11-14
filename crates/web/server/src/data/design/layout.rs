//! Canvas layout hints: where the editor's user placed bricks, by brick path (`root/web/api`).
//! Stored in `.brickie/layout.json` beside the history, outside the versioned design: a
//! position is a view preference, not a design change, so it never makes a version, never
//! goes stale and is not undone. A hint whose brick is gone is ignored.

use anyhow::Result;
use brickie_web_api::LayoutHint;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type Hints = BTreeMap<String, LayoutHint>;

fn file(root: &Path) -> PathBuf {
    root.join(".brickie/layout.json")
}

/// Every stored hint (none when the file is missing or unreadable).
pub fn read_layout(root: &Path) -> Hints {
    std::fs::read(file(root))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write(root: &Path, hints: &Hints) -> Result<()> {
    let path = file(root);
    std::fs::create_dir_all(root.join(".brickie"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(hints)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Set (`Some`) or clear the hint of brick `brick`.
pub fn set_hint(root: &Path, brick: &str, hint: Option<LayoutHint>) -> Result<()> {
    let mut hints = read_layout(root);
    match hint {
        Some(h) => hints.insert(brick.to_string(), h),
        None => hints.remove(brick),
    };
    write(root, &hints)
}

/// Clear the hints of every brick directly on `level` (`root`, `root/web`).
pub fn clear_level(root: &Path, level: &str) -> Result<()> {
    let mut hints = read_layout(root);
    hints.retain(|k, _| k.rsplit_once('/').is_none_or(|(p, _)| p != level));
    write(root, &hints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_clears_only_its_own_bricks() {
        let dir = tempfile::tempdir().unwrap();
        let h = LayoutHint { x: 1.0, y: 2.0 };
        for b in ["root/a", "root/b", "root/a/x"] {
            set_hint(dir.path(), b, Some(h)).unwrap();
        }
        clear_level(dir.path(), "root").unwrap();
        let left: Vec<String> = read_layout(dir.path()).into_keys().collect();
        assert_eq!(left, vec!["root/a/x"]);
        set_hint(dir.path(), "root/a/x", None).unwrap();
        assert!(read_layout(dir.path()).is_empty());
    }
}
