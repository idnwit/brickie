//! Runs `rust-analyzer scip` once per measure.

use anyhow::{Context, Result, bail};
use std::io::Read;
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
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .context("failed to start rust-analyzer (is it on PATH?)")?;
    let pgid = child.id();
    // Drain stderr on a thread so a chatty child never blocks on a full pipe.
    let mut pipe = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });
    let status = child.wait();
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .stderr(Stdio::null())
        .status();
    let stderr = reader.join().unwrap_or_default();
    if !status?.success() {
        let tail = last_lines(&stderr, 20);
        if tail.is_empty() {
            bail!("rust-analyzer scip failed");
        }
        bail!("rust-analyzer scip failed; its last output:\n{tail}");
    }
    Ok(())
}

/// The last `n` non-empty lines of `text`.
fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::last_lines;

    #[test]
    fn keeps_only_the_last_lines() {
        let text: String = (1..=30).map(|i| format!("l{i}\n")).collect();
        let tail = last_lines(&text, 20);
        assert_eq!(tail.lines().count(), 20);
        assert!(tail.starts_with("l11") && tail.ends_with("l30"));
        assert_eq!(last_lines("\n \n", 20), "");
    }
}
