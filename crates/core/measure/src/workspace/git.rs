//! Git facts recorded in the cache.

use brickie_facts::Git;
use std::path::Path;
use std::process::{Command, Stdio};

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn git_info(root: &Path) -> Option<Git> {
    let commit = git(root, &["rev-parse", "HEAD"])?;
    let dirty = git(root, &["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    Some(Git { commit, dirty })
}
