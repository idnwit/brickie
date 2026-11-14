//! Runs `rust-analyzer scip` for the host and for each additional target.

use anyhow::{Context, Result, bail};
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// The `--config-path` JSON: every cargo feature, and `target` when it is not the host.
/// rust-analyzer reads the nested form only; flat `cargo.features` keys are ignored.
pub fn config_json(target: Option<&str>) -> String {
    let mut cargo = serde_json::json!({ "features": "all" });
    if let Some(t) = target {
        cargo["target"] = t.into();
    }
    serde_json::json!({ "cargo": cargo }).to_string()
}

/// The error of a failed run: names the target and the likely cause of a failure with all
/// features on.
pub fn failure_message(target: Option<&str>, stderr: &str) -> String {
    let which = target.map_or("the host target".to_string(), |t| format!("target {t}"));
    let tail = last_lines(stderr, 20);
    let mut msg = format!("rust-analyzer scip failed for {which}");
    if !tail.is_empty() {
        msg += &format!("; its last output:\n{tail}");
    }
    msg += "\nif features are mutually exclusive, make them additive (a build must work with --all-features)";
    msg
}

/// Run `rust-analyzer scip` (all features, `target` or the host) in its own process group,
/// wait for it, then kill whatever is left of the group so no rust-analyzer process survives.
pub fn run(root: &Path, out: &Path, target: Option<&str>) -> Result<()> {
    let config = out.with_extension("config.json");
    std::fs::write(&config, config_json(target)).context("write rust-analyzer config")?;
    // cwd = workspace so rustup resolves the workspace's own toolchain.
    let mut child = Command::new("rust-analyzer")
        .current_dir(root)
        .arg("scip")
        .arg(root)
        .arg("--config-path")
        .arg(&config)
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
        bail!("{}", failure_message(target, &stderr));
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
    use super::{config_json, failure_message, last_lines};

    #[test]
    fn keeps_only_the_last_lines() {
        let text: String = (1..=30).map(|i| format!("l{i}\n")).collect();
        let tail = last_lines(&text, 20);
        assert_eq!(tail.lines().count(), 20);
        assert!(tail.starts_with("l11") && tail.ends_with("l30"));
        assert_eq!(last_lines("\n \n", 20), "");
    }

    #[test]
    fn config_is_nested_and_names_the_target() {
        let v: serde_json::Value = serde_json::from_str(&config_json(None)).unwrap();
        assert_eq!(v["cargo"]["features"], "all");
        assert!(v["cargo"].get("target").is_none());
        let v: serde_json::Value =
            serde_json::from_str(&config_json(Some("wasm32-unknown-unknown"))).unwrap();
        assert_eq!(v["cargo"]["target"], "wasm32-unknown-unknown");
    }

    #[test]
    fn failure_names_target_and_explains_features() {
        let m = failure_message(Some("wasm32-unknown-unknown"), "boom\n");
        assert!(
            m.contains("target wasm32-unknown-unknown") && m.contains("boom"),
            "{m}"
        );
        assert!(m.contains("make them additive (a build must work with --all-features)"));
        let m = failure_message(None, "");
        assert!(
            m.contains("host target") && !m.contains("last output"),
            "{m}"
        );
    }
}
