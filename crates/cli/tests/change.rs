//! `brickie change`: open -> propose -> reject -> propose -> accept -> publish on a temporary
//! workspace, and `check --change` over the preview.

use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_brickie");

fn run(dir: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(stdin.unwrap_or("").as_bytes()).unwrap();
    drop(input);
    child.wait_with_output().unwrap()
}

fn json(dir: &Path, args: &[&str], stdin: Option<&str>) -> Value {
    let mut all = args.to_vec();
    all.extend(["--format", "json"]);
    let out = run(dir, &all, stdin);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{args:?}: {err}");
    serde_json::from_slice(&out.stdout).unwrap()
}

const ROOT: &str = "# design\n[[brick]]\nname = \"a\"\nmembers = [\"pa\"]\n";

#[test]
fn open_propose_review_publish() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("brickie.toml"), ROOT).unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    let none = run(dir, &["change"], None);
    assert_eq!(none.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&none.stderr).contains("change/none"));

    json(dir, &["change", "open", "--intent", "add a store"], None);
    let op = r#"{"op":"add_brick","file":"brickie.toml","brick":{"name":"b","members":["path:crates/b/"]}}"#;
    let r = json(
        dir,
        &["change", "propose", "--json", "--note", "storage"],
        Some(op),
    );
    assert_eq!(
        (r["seq"].as_u64(), r["state"].as_str()),
        (Some(1), Some("proposed"))
    );
    assert_eq!(r["after"].as_str().unwrap().lines().count(), 2);

    // Publishing is refused while an op is proposed.
    let refused = run(dir, &["change", "publish"], None);
    assert_eq!(refused.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("change/proposed"));

    let r = json(
        dir,
        &["change", "reject", "1", "--note", "call it store"],
        None,
    );
    assert_eq!(r["target"].as_u64(), Some(1));
    let op = op
        .replace("\"b\"", "\"store\"")
        .replace("crates/b/", "crates/store/");
    json(dir, &["change", "propose", "--json"], Some(&op));
    json(dir, &["change", "accept", "3"], None);

    // The agent sees everything once, with the rejection note.
    let show = json(dir, &["change"], None);
    assert_eq!(show["new"]["entries"].as_array().unwrap().len(), 4);
    assert_eq!(show["status"]["accepted"].as_u64(), Some(1));
    let text = serde_json::to_string(&show["diff"]).unwrap();
    assert!(text.contains("call it store"), "{text}");
    assert!(
        json(dir, &["change"], None)["new"]["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let human = String::from_utf8(run(dir, &["change", "--as", "human"], None).stdout).unwrap();
    assert!(human.contains("new since #0: 4"), "{human}");

    let todo = json(dir, &["change", "--todo"], None);
    assert!(
        todo.as_array()
            .unwrap()
            .iter()
            .any(|t| t["kind"] == "rejected")
    );

    let plan = json(dir, &["change", "publish", "--dry-run"], None);
    assert!(plan["blockers"].as_array().unwrap().is_empty(), "{plan}");
    assert_eq!(
        std::fs::read_to_string(dir.join("brickie.toml")).unwrap(),
        ROOT
    );

    json(dir, &["change", "publish"], None);
    let cfg = std::fs::read_to_string(dir.join("brickie.toml")).unwrap();
    assert!(
        cfg.starts_with("# design") && cfg.contains("name = \"store\""),
        "{cfg}"
    );
    assert!(dir.join("crates/store/src/lib.rs").is_file());
    let ws = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(ws.contains("crates/store"), "{ws}");
    assert!(!dir.join(".brickie/change").exists());
}

#[test]
fn check_change_reports_planned_bricks_of_the_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("brickie.toml"), ROOT).unwrap();
    std::fs::create_dir_all(dir.join(".brickie")).unwrap();
    let cache = serde_json::json!({
        "schema_version": 6, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": dir.display().to_string(), "git": null,
        "crates": [{"id": "pa", "package": "pa", "version": "0", "target": "pa", "kind": "lib",
                    "root": "crates/pa/src/lib.rs", "dir": "crates/pa"}],
        "modules": [{"id": "pa", "crate": "pa", "file": "crates/pa/src/lib.rs", "line": 1}],
        "files": {},
    });
    std::fs::write(dir.join(".brickie/cache.json"), cache.to_string()).unwrap();
    json(dir, &["change", "open", "--intent", "x"], None);
    let op = r#"{"op":"add_brick","file":"brickie.toml","brick":{"name":"b","members":["pb"]}}"#;
    json(
        dir,
        &["change", "propose", "--json", "--as", "human"],
        Some(op),
    );
    let out = run(dir, &["check", "--change"], None);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("note[change/planned]: brick root/b"),
        "{text}"
    );
    assert!(!text.contains("root/a"), "{text}");
    let plain = run(dir, &["check"], None);
    assert!(!String::from_utf8_lossy(&plain.stdout).contains("change/planned"));
}
