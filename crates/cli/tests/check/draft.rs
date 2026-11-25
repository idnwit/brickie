//! Check tests: `check` reads the working design, the draft when one exists.

use super::nested::{ROOT, WEB, tree};
use super::support::*;
use serde_json::json;
use std::process::Command;

/// A planned group brick whose folder (and `brickie.toml`) does not exist yet: a draft edit.
fn add_ghost(config: &std::path::Path) {
    let o = Command::new(BIN)
        .args([
            "design",
            "brick",
            "add",
            "ghost",
            "--kind",
            "composite",
            "--base",
            "1",
        ])
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn check_reads_the_draft_when_one_exists() {
    let w = tree();
    w.write("ws/crates/web/brickie.toml", WEB);
    let path = w.save(ROOT);
    // Without a draft: the published files, and the header says so.
    let o = run(&path, &["--format", "short"]);
    assert!(o.stdout.starts_with("working: published"), "{}", o.stdout);
    assert!(!o.stdout.contains("draft"), "{}", o.stdout);

    add_ghost(&path);
    let o = run(&path, &["--format", "json"]);
    assert!(
        o.json["working"].as_str().unwrap().starts_with("draft"),
        "{}",
        o.json
    );
    // The missing nested file is planned work, not `config/missing`.
    assert!(codes(&o, "config/").is_empty(), "{}", o.stdout);
    assert_eq!(o.json["todo"]["planned_bricks"], json!(["root/ghost"]));
    let o = run(&path, &["--format", "short"]);
    assert!(
        o.stdout.starts_with("working: draft v") && o.stdout.contains("from published v1"),
        "{}",
        o.stdout
    );

    // `interface` reads the same source.
    let o = Command::new(BIN)
        .args(["interface", "ghost", "--config"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
