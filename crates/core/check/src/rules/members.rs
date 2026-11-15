//! Workspace membership as a staleness signal: a member crate the cache does not know (added
//! after `measure`) or a recorded package directory that lost its `Cargo.toml` (removed).
//!
//! The members come from the root `Cargo.toml` (`[workspace] members` and `exclude`) expanded
//! on disk, never from cargo, so `check` stays fast. A directory with a `Cargo.toml` that is not
//! a member is ignored: re-measuring would not make it go away. Member patterns support `*` and
//! `?` inside a path segment; any other pattern form is skipped rather than guessed.

use std::collections::BTreeSet;
use std::path::Path;

/// A package directory that differs from the cache: `(path of its Cargo.toml, added)`.
pub(super) fn member_changes(root: &Path, known: &BTreeSet<String>) -> Vec<(String, bool)> {
    let Some((members, exclude)) = workspace_lists(root) else {
        return Vec::new();
    };
    let mut dirs = BTreeSet::new();
    for pattern in &members {
        if let Some(found) = expand(root, pattern) {
            dirs.extend(found);
        }
    }
    let mut out = Vec::new();
    for dir in dirs {
        let excluded = exclude
            .iter()
            .any(|e| dir == *e || dir.starts_with(&format!("{e}/")));
        if !excluded && !known.contains(&dir) && root.join(&dir).join("Cargo.toml").is_file() {
            out.push((manifest(&dir), true));
        }
    }
    for dir in known {
        if !root.join(dir).join("Cargo.toml").is_file() {
            out.push((manifest(dir), false));
        }
    }
    out
}

fn manifest(dir: &str) -> String {
    if dir.is_empty() {
        "Cargo.toml".into()
    } else {
        format!("{dir}/Cargo.toml")
    }
}

/// `members` and `exclude` of the root manifest; `None` without a `[workspace]` table.
fn workspace_lists(root: &Path) -> Option<(Vec<String>, Vec<String>)> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let value: toml::Value = text.parse().ok()?;
    let ws = value.get("workspace")?;
    let list = |key: &str| -> Vec<String> {
        ws.get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str())
            .map(|s| s.trim_start_matches("./").trim_end_matches('/').to_string())
            .collect()
    };
    Some((list("members"), list("exclude")))
}

/// Directories (relative, `/`-separated) matching `pattern`; `None` for an unsupported form.
fn expand(root: &Path, pattern: &str) -> Option<Vec<String>> {
    let mut dirs = vec![String::new()];
    for seg in pattern.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || seg.contains(['[', ']', '{', '}', '\\']) {
            return None;
        }
        if seg.contains("**") || seg.contains(':') {
            return None;
        }
        let mut next = Vec::new();
        for base in &dirs {
            if seg.contains(['*', '?']) {
                next.extend(matching_children(root, base, seg));
            } else {
                next.push(join(base, seg));
            }
        }
        dirs = next;
    }
    Some(dirs)
}

fn join(base: &str, seg: &str) -> String {
    if base.is_empty() {
        seg.to_string()
    } else {
        format!("{base}/{seg}")
    }
}

fn matching_children(root: &Path, base: &str, pat: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root.join(base)) else {
        return Vec::new();
    };
    rd.flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| wildcard(pat.as_bytes(), n.as_bytes()))
        .map(|n| join(base, &n))
        .collect()
}

/// `*` matches any run of bytes, `?` any one byte.
fn wildcard(p: &[u8], s: &[u8]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((b'*', rest)) => (0..=s.len()).any(|i| wildcard(rest, &s[i..])),
        Some((b'?', rest)) => !s.is_empty() && wildcard(rest, &s[1..]),
        Some((c, rest)) => s.first() == Some(c) && wildcard(rest, &s[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(root: &Path, dir: &str) {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        std::fs::write(root.join(dir).join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    }

    fn setup(workspace: &str, dirs: &[&str]) -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("Cargo.toml"), workspace).unwrap();
        for d in dirs {
            krate(t.path(), d);
        }
        t
    }

    fn known(dirs: &[&str]) -> BTreeSet<String> {
        dirs.iter().map(|d| d.to_string()).collect()
    }

    const GLOB: &str = "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/skip\"]\n";

    #[test]
    fn new_member_under_glob_is_added() {
        let t = setup(GLOB, &["crates/a", "crates/rogue"]);
        let c = member_changes(t.path(), &known(&["crates/a"]));
        assert_eq!(c, vec![("crates/rogue/Cargo.toml".to_string(), true)]);
    }

    #[test]
    fn literal_member_is_added() {
        let t = setup("[workspace]\nmembers = [\"tools/x\"]\n", &["tools/x"]);
        let c = member_changes(t.path(), &known(&[]));
        assert_eq!(c, vec![("tools/x/Cargo.toml".to_string(), true)]);
    }

    #[test]
    fn excluded_and_non_member_dirs_are_ignored() {
        let t = setup(
            GLOB,
            &["crates/a", "crates/skip", "other/y", "crates/a/inner"],
        );
        assert!(member_changes(t.path(), &known(&["crates/a"])).is_empty());
    }

    #[test]
    fn removed_member_is_deleted() {
        let t = setup(GLOB, &["crates/a"]);
        let c = member_changes(t.path(), &known(&["crates/a", "crates/gone"]));
        assert_eq!(c, vec![("crates/gone/Cargo.toml".to_string(), false)]);
    }

    #[test]
    fn no_workspace_or_unsupported_pattern_does_nothing() {
        let t = setup("[package]\nname = \"p\"\n", &["crates/new"]);
        assert!(member_changes(t.path(), &known(&[])).is_empty());
        let t = setup(
            "[workspace]\nmembers = [\"crates/**\", \"c/[ab]\"]\n",
            &["crates/n", "c/a"],
        );
        assert!(member_changes(t.path(), &known(&[])).is_empty());
    }
}
