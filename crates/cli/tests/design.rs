//! `brickie design` on a temporary workspace: every verb, the stale exit code and the JSON of
//! `show` against a golden file (`BRICKIE_UPDATE_SNAPSHOTS=1` rewrites it).

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_brickie");

const ROOT: &str = "# design\n[[brick]]\nname = \"a\"   # first\nmembers = [\"pa\"]\n\n\
[[brick]]\nname = \"b\"\nmembers = [\"pb\"]\n\n[deps]\ndefault = \"deny\"\n\n\
[[deps.rule]]\nfrom = \"a\"\nto = \"b\"\naction = \"allow\"\nreason = \"a reads b\"\n";

fn ws() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("brickie.toml"), ROOT).unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    tmp
}

fn run(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["design"];
    all.extend(args);
    Command::new(BIN)
        .args(all)
        .current_dir(dir)
        .env_remove("BRICKIE_BASE")
        .output()
        .unwrap()
}

/// Run a write on base `v`; expect `ok: v<v+1>`.
fn ok(dir: &Path, v: u64, args: &[&str]) {
    let base = v.to_string();
    let mut all = args.to_vec();
    all.extend(["--base", &base]);
    let out = run(dir, &all);
    let text = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{args:?}: {err}");
    assert!(
        text.starts_with(&format!("ok: v{}", v + 1)),
        "{args:?}: {text}"
    );
}

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

fn json(dir: &Path, args: &[&str]) -> Value {
    let mut all = args.to_vec();
    all.extend(["--format", "json"]);
    let out = run(dir, &all);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn every_write_verb() {
    let tmp = ws();
    let d = tmp.path();
    let show = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    assert!(show.starts_with("design v1 "), "{show}");
    ok(
        d,
        1,
        &["brick", "add", "store", "--kind", "crate", "--desc", "data"],
    );
    assert!(d.join("crates/store/src/lib.rs").is_file(), "scaffolded");
    ok(d, 2, &["brick", "set", "store", "--tags", "io,db"]);
    ok(d, 3, &["brick", "rename", "store", "db"]);
    ok(
        d,
        4,
        &[
            "rule", "add", "a", "db", "allow", "--kind", "uses", "--reason", "x",
        ],
    );
    ok(d, 5, &["rule", "set", "a", "db", "--action", "deny"]);
    ok(d, 6, &["rule", "rm", "a", "db", "--kind", "uses"]);
    ok(d, 7, &["limit", "add", "--cognitive", "20"]);
    ok(
        d,
        8,
        &["limit", "set", "*", "--cognitive", "0", "--lines", "80"],
    );
    let text = read(d, "brickie.toml");
    assert!(
        text.contains("lines = 80") && !text.contains("cognitive"),
        "{text}"
    );
    ok(d, 9, &["limit", "rm"]);
    ok(d, 10, &["deps", "set", "--cycles", "deny"]);
    std::fs::write(d.join("preset.toml"), "[deps]\ndefault = \"deny\"\n").unwrap();
    ok(d, 11, &["include", "add", "preset.toml"]);
    ok(d, 12, &["include", "rm", "preset.toml"]);
    ok(d, 13, &["brick", "group", "app", "a", "b"]);
    assert!(read(d, "app/brickie.toml").contains("name = \"a\"   # first"));
    ok(d, 14, &["export", "set", "app/a"]);
    ok(d, 15, &["export", "rm", "app/a"]);
    ok(d, 16, &["brick", "move", "app/b", "--into", "root"]);
    ok(d, 17, &["brick", "ungroup", "app"]);
    ok(d, 18, &["brick", "rm", "db"]);
    let log = json(d, &["log"]);
    assert_eq!(log.as_array().unwrap().len(), 19);
    assert!(
        log.as_array().unwrap()[1..]
            .iter()
            .all(|v| v["by"] == "agent")
    );
    // undo, diff, show --since
    ok(d, 19, &["undo"]);
    assert!(read(d, "brickie.toml").contains("name = \"db\""));
    let diff = String::from_utf8_lossy(&run(d, &["diff", "1", "2"]).stdout).to_string();
    assert_eq!(diff, "+ brickie.toml: brick store\n");
    let since = String::from_utf8_lossy(&run(d, &["show", "--since", "19"]).stdout).to_string();
    assert!(
        since.contains("v20  agent") && since.contains("undo v19"),
        "{since}"
    );
}

#[test]
fn stale_base_exits_3_and_writes_nothing() {
    let tmp = ws();
    let d = tmp.path();
    ok(d, 1, &["brick", "set", "a", "--desc", "x"]);
    let out = run(d, &["brick", "set", "b", "--desc", "y", "--base", "1"]);
    assert_eq!(out.status.code(), Some(3));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("error[design/stale]: design is at v2 (by agent"),
        "{err}"
    );
    assert!(
        err.contains("~ brickie.toml: brick a: desc: (none) -> \"x\""),
        "{err}"
    );
    assert!(!read(d, "brickie.toml").contains("\"y\""));
    // BRICKIE_BASE is the fallback for --base.
    let env = Command::new(BIN)
        .args(["design", "brick", "set", "b", "--desc", "y"])
        .current_dir(d)
        .env("BRICKIE_BASE", "2")
        .output()
        .unwrap();
    assert!(
        env.status.success(),
        "{}",
        String::from_utf8_lossy(&env.stderr)
    );
    // A missing rule is an error (exit 2) naming the candidates.
    let out = run(d, &["rule", "rm", "b", "a", "--base", "3"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("design/no-match"));
}

#[test]
fn show_json_matches_the_golden_file() {
    let tmp = ws();
    let d = tmp.path();
    ok(d, 1, &["brick", "add", "store", "--kind", "crate"]);
    ok(
        d,
        2,
        &["limit", "add", "--cognitive", "20", "--level", "warn"],
    );
    let mut v = json(d, &["show"]);
    v["at"] = Value::from(0);
    let got = serde_json::to_string_pretty(&v).unwrap() + "\n";
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/design_show.json");
    if std::env::var_os("BRICKIE_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&file, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(&file).unwrap());
}
