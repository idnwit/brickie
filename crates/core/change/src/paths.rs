//! Workspace-relative paths: `/`-separated, relative to the root config's directory.

use std::path::{Component, Path, PathBuf};

/// Resolve `.` and `..` without touching the file system.
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `rel` resolved against the directory of `file` (both workspace-relative).
pub fn join_rel(file: &str, rel: &str) -> String {
    let dir = Path::new(file).parent().unwrap_or(Path::new(""));
    slash(&normalize(&dir.join(rel)))
}

/// `rel` resolved against the workspace-relative directory `dir`.
pub fn join_dir(dir: &str, rel: &str) -> String {
    slash(&normalize(&Path::new(dir).join(rel)))
}

/// `path` (workspace-relative) as seen from directory `dir` (`..` where needed).
pub fn relative(dir: &str, path: &str) -> String {
    let d: Vec<&str> = dir.split('/').filter(|c| !c.is_empty()).collect();
    let p: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
    let common = d.iter().zip(&p).take_while(|(a, b)| a == b).count();
    let mut out: Vec<&str> = vec![".."; d.len() - common];
    out.extend(&p[common..]);
    if out.is_empty() {
        ".".into()
    } else {
        out.join("/")
    }
}

/// The directory of a workspace-relative file (`""` at the root).
pub fn dir_of(file: &str) -> String {
    file.rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default()
}

/// `p` relative to `root`, `/`-separated; `None` when outside.
pub fn to_rel(root: &Path, p: &Path) -> Option<String> {
    normalize(p).strip_prefix(root).ok().map(slash)
}

pub fn slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_relative_paths() {
        assert_eq!(join_rel("brickie.toml", "presets/a.toml"), "presets/a.toml");
        assert_eq!(
            join_rel("crates/x/brickie.toml", "../p.toml"),
            "crates/p.toml"
        );
        assert_eq!(
            join_rel("a/brickie.toml", "./b/brickie.toml"),
            "a/b/brickie.toml"
        );
        assert_eq!(dir_of("a/b.toml"), "a");
        assert_eq!(dir_of("b.toml"), "");
        assert_eq!(relative("crates/web", "crates/core/x"), "../core/x");
        assert_eq!(relative("", "crates/x"), "crates/x");
    }
}
