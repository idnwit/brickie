//! Runs `rust-analyzer scip` once per measure.

use anyhow::{Context, Result, bail};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Run `rust-analyzer scip` once in its own process group, wait for it, then
/// kill whatever is left of the group so no rust-analyzer process survives.
pub fn run(root: &Path, out: &Path) -> Result<()> {
    // cwd = workspace so rustup resolves the workspace's own toolchain.
    let mut child = Command::new("rust-analyzer")
        .current_dir(root)
        .arg("scip")
        .arg(root)
        .arg("--output")
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .process_group(0)
        .spawn()
        .context("failed to start rust-analyzer (is it on PATH?)")?;
    let pgid = child.id();
    let status = child.wait();
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .stderr(Stdio::null())
        .status();
    if !status?.success() {
        bail!("rust-analyzer scip failed");
    }
    Ok(())
}
