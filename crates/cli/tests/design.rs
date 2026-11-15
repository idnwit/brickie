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
    assert!(
        !d.join("crates/store").exists(),
        "no code without --scaffold"
    );
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

fn apply(dir: &Path, args: &[&str], script: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(BIN)
        .arg("design")
        .arg("apply")
        .args(args)
        .current_dir(dir)
        .env_remove("BRICKIE_BASE")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

const SCRIPT: &str = "# one task\n\nbrick add store --kind crate --desc \"the data\"\n\
rule add a store allow --reason \"a reads store\"\n";

#[test]
fn apply_writes_all_ops_as_one_version() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    let out = apply(d, &["--base", "1", "--note", "add store"], SCRIPT);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("ok: v2"));
    let log = json(d, &["log"]);
    assert_eq!(log.as_array().unwrap().len(), 2, "one version");
    assert_eq!(log[1]["note"], "add store");
    let text = read(d, "brickie.toml");
    assert!(text.contains("name = \"store\"") && text.contains("to = \"store\""));
}

#[test]
fn apply_parse_error_names_the_line_and_writes_nothing() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    let before = read(d, "brickie.toml");
    let script = "brick add store\n\nrule add a store sideways\n";
    let out = apply(d, &["--base", "1"], script);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("line 3"), "{err}");
    assert_eq!(read(d, "brickie.toml"), before);
    assert_eq!(json(d, &["log"]).as_array().unwrap().len(), 1);
    // a per-line --base or --note is refused too
    let out = apply(d, &["--base", "1"], "brick add x --note hi\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("line 1") && !out.status.success(), "{err}");
    let out = apply(d, &["--base", "1"], "brick add x --base 1\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("line 1"));
}

#[test]
fn dry_run_prints_the_diff_and_writes_nothing() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    let before = read(d, "brickie.toml");
    let out = apply(d, &["--base", "1", "--dry-run"], SCRIPT);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("dry run on v1 (nothing written)") && text.contains("brick store"),
        "{text}"
    );
    let out = apply(d, &["--base", "1", "--dry-run"], "rule add b a allow\n");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("new todo") && text.contains("no-reason"),
        "{text}"
    );
    assert_eq!(read(d, "brickie.toml"), before);
    assert!(!d.join("crates/store").exists(), "no scaffold");
    assert_eq!(json(d, &["log"]).as_array().unwrap().len(), 1);
    // a write subcommand takes it too
    let out = run(
        d,
        &[
            "brick",
            "set",
            "b",
            "--desc",
            "d",
            "--base",
            "1",
            "--dry-run",
        ],
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("dry run"),
        "{err}"
    );
    assert_eq!(read(d, "brickie.toml"), before);
    // a stale base still fails
    let out = apply(d, &["--base", "0", "--dry-run"], SCRIPT);
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn show_starts_with_the_versions_after_the_agents_last() {
    let tmp = ws();
    let d = tmp.path();
    let plain = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    assert!(plain.starts_with("design v1"), "no agent version: {plain}");
    ok(d, 1, &["brick", "set", "a", "--desc", "x"]);
    let text = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    assert!(text.starts_with("design v2"), "nothing after agent: {text}");
    assert!(json(d, &["show"]).get("since_agent").is_none());
    let file = read(d, "brickie.toml") + "\n[[brick]]\nname = \"c\"\nmembers = [\"pc\"]\n";
    std::fs::write(d.join("brickie.toml"), file).unwrap();
    let text = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    assert!(
        text.starts_with("since your last version (agent):\nv3  external"),
        "{text}"
    );
    assert!(
        text.contains("brick c") && text.contains("design v3"),
        "{text}"
    );
    let v = json(d, &["show"]);
    assert_eq!(v["since_agent"][0]["v"], 3);
}

#[test]
fn show_in_prints_only_that_level() {
    let tmp = ws();
    let d = tmp.path();
    ok(d, 1, &["brick", "group", "app", "a", "b"]);
    let full = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    let part = String::from_utf8_lossy(&run(d, &["show", "--in", "app"]).stdout).to_string();
    assert!(full.contains("root (brickie.toml)") && !part.contains("root (brickie.toml)"));
    assert!(
        part.contains("root/app") && part.contains("brick a "),
        "{part}"
    );
    let v = json(d, &["show", "--in", "app"]);
    assert_eq!(v["root"]["path"], "root/app");
    let out = run(d, &["show", "--in", "app/a"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!run(d, &["show", "--in", "nope"]).status.success());
}

#[test]
fn a_deleted_nested_file_does_not_block_show_log_or_undo() {
    let tmp = ws();
    let d = tmp.path();
    ok(d, 1, &["brick", "add", "dom", "--kind", "group"]);
    let nested = read(d, "dom/brickie.toml");
    std::fs::remove_file(d.join("dom/brickie.toml")).unwrap();
    let out = run(d, &["show"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("warning: missing file dom/brickie.toml"),
        "{text}"
    );
    assert!(run(d, &["log"]).status.success());
    ok(d, 3, &["undo"]);
    assert_eq!(read(d, "dom/brickie.toml"), nested);
    let text = String::from_utf8_lossy(&run(d, &["show"]).stdout).to_string();
    assert!(!text.contains("warning"), "{text}");
}

#[test]
fn set_path_creates_the_nested_file_and_never_overwrites() {
    let tmp = ws();
    let d = tmp.path();
    let out = run(d, &["brick", "set", "a", "--path", "ga", "--base", "1"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(d.join("ga/brickie.toml").is_file());
    std::fs::create_dir_all(d.join("gb")).unwrap();
    std::fs::write(d.join("gb/brickie.toml"), "# mine\n").unwrap();
    ok(d, 2, &["brick", "set", "b", "--path", "gb"]);
    assert_eq!(read(d, "gb/brickie.toml"), "# mine\n");
}

#[test]
fn add_and_group_print_the_chosen_folder() {
    let tmp = ws();
    let d = tmp.path();
    let out = run(
        d,
        &["brick", "add", "dom", "--kind", "group", "--base", "1"],
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("created dom/ (group)"), "{text}");
    let out = run(d, &["brick", "group", "gg", "a", "b", "--base", "2"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("created gg/ (group)"), "{text}");
}

/// The exported brick names of `app/brickie.toml`, sorted (the array may span lines).
fn app_exports(d: &Path) -> Vec<String> {
    let text = read(d, "app/brickie.toml");
    let start = text.find("exports = [").map_or(0, |i| i + 10);
    let body = &text[start
        ..text[start..]
            .find("\n]")
            .map_or(text.len(), |e| start + e + 2)];
    let body = body
        .lines()
        .next()
        .filter(|l| l.contains(']'))
        .unwrap_or(body);
    let mut names = Vec::new();
    for part in body.split(&[',', '{', '}', '[', ']'][..]) {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("brick = ") {
            names.push(v.trim_matches('"').to_string());
        } else if part.starts_with('"') {
            names.push(part.trim_matches('"').to_string());
        }
    }
    names.sort();
    names
}

fn grouped() -> tempfile::TempDir {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    ok(
        d,
        1,
        &["brick", "add", "c", "--kind", "crate", "--desc", "c"],
    );
    let out = apply(d, &["--base", "2"], "brick group app a b c\n");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    tmp
}

#[test]
fn apply_keeps_every_export_set_line() {
    let tmp = grouped();
    let d = tmp.path();
    let script = "export set app/a\nexport set app/b --scope prod\nexport set app/c\n";
    let out = apply(d, &["--base", "3"], script);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(app_exports(d), ["a", "b", "c"]);
    assert!(read(d, "app/brickie.toml").contains("scope = \"prod\""));
}

#[test]
fn apply_mixes_export_set_and_rm() {
    let tmp = grouped();
    let d = tmp.path();
    let out = apply(d, &["--base", "3"], "export set app/a\nexport set app/b\n");
    assert!(out.status.success());
    let script = "export rm app/a\nexport set app/c\nexport set app/a\nexport rm app/b\n";
    let out = apply(d, &["--base", "4"], script);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(app_exports(d), ["a", "c"]);
}

#[test]
fn apply_keeps_every_export_rm_line() {
    let tmp = grouped();
    let d = tmp.path();
    let out = apply(
        d,
        &["--base", "3"],
        "export set app/a\nexport set app/b\nexport set app/c\n",
    );
    assert!(out.status.success());
    let out = apply(d, &["--base", "4"], "export rm app/a\nexport rm app/c\n");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(app_exports(d), ["b"]);
}

/// Every path of the tree (files and folders) except `.brickie`, sorted.
fn tree(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            if rel.starts_with(".brickie") {
                continue;
            }
            out.push(rel);
            if p.is_dir() {
                walk(base, &p, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn brick_add_writes_no_code_unless_scaffold_is_given() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    let before = tree(d);
    ok(d, 1, &["brick", "add", "x", "--kind", "crate"]);
    assert_eq!(tree(d), before, "only brickie.toml changed");
    assert_eq!(read(d, "Cargo.toml"), "[workspace]\nmembers = []\n");
    // --dry-run shows no code plan without the flag, and the plan with it.
    let plan = run(
        d,
        &[
            "brick",
            "add",
            "y",
            "--kind",
            "crate",
            "--base",
            "2",
            "--dry-run",
        ],
    );
    assert!(!String::from_utf8_lossy(&plan.stdout).contains("would scaffold"));
    let plan = run(
        d,
        &[
            "brick",
            "add",
            "y",
            "--kind",
            "crate",
            "--scaffold",
            "--base",
            "2",
            "--dry-run",
        ],
    );
    assert!(String::from_utf8_lossy(&plan.stdout).contains("would scaffold crates/y/src/lib.rs"));
    ok(
        d,
        2,
        &["brick", "add", "y", "--kind", "crate", "--scaffold"],
    );
    assert!(d.join("crates/y/src/lib.rs").is_file());
    assert!(read(d, "Cargo.toml").contains("crates/y"));
    assert!(!d.join("crates/x").exists());
    // undo reverts the design and leaves the scaffolded code.
    ok(d, 3, &["undo"]);
    assert!(d.join("crates/y/src/lib.rs").is_file());
}

#[test]
fn apply_scaffold_covers_every_line_and_is_refused_inside_one() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    let script = "brick add x --kind crate\nbrick add y --kind crate\n";
    assert!(apply(d, &["--base", "1"], script).status.success());
    assert!(!d.join("crates/x").exists() && !d.join("crates/y").exists());
    let script = "brick add p --kind crate\nbrick add q --kind crate\n";
    assert!(
        apply(d, &["--base", "2", "--scaffold"], script)
            .status
            .success()
    );
    assert!(d.join("crates/p/src/lib.rs").is_file() && d.join("crates/q/src/lib.rs").is_file());
    let out = apply(d, &["--base", "3"], "brick add z --kind crate --scaffold\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("line 1"));
    // `--scaffold` means nothing for other commands.
    assert!(
        !run(
            d,
            &[
                "rule",
                "add",
                "a",
                "b",
                "allow",
                "--scaffold",
                "--base",
                "3"
            ]
        )
        .status
        .success()
    );
}

#[test]
fn a_new_group_is_as_strict_as_its_parent() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    ok(d, 1, &["brick", "add", "grp", "--kind", "group"]);
    assert!(read(d, "grp/brickie.toml").contains("default = \"deny\""));
    ok(d, 2, &["deps", "set", "--cycles", "deny"]);
    ok(d, 3, &["brick", "group", "g2", "a", "b"]);
    let text = read(d, "g2/brickie.toml");
    assert!(
        text.contains("default = \"deny\"") && text.contains("cycles = \"deny\""),
        "{text}"
    );
    ok(d, 4, &["brick", "add", "k", "--members", "pk"]);
    ok(d, 5, &["brick", "set", "k", "--path", "ga"]);
    assert!(read(d, "ga/brickie.toml").contains("default = \"deny\""));
}

#[test]
fn a_rule_that_changes_nothing_prints_a_note() {
    let tmp = ws();
    let d = tmp.path();
    run(d, &["show"]);
    ok(d, 1, &["brick", "add", "grp", "--kind", "group"]);
    // A nested file written by hand without `deps` is as loose as a level can be.
    std::fs::write(d.join("grp/brickie.toml"), "# loose\n").unwrap();
    ok(
        d,
        3,
        &["brick", "add", "m", "--members", "pm", "--in", "grp"],
    );
    ok(
        d,
        4,
        &["brick", "add", "n", "--members", "pn", "--in", "grp"],
    );
    let args = [
        "rule", "add", "m", "n", "allow", "--in", "grp", "--base", "5",
    ];
    let text = String::from_utf8_lossy(&run(d, &args).stdout).to_string();
    let want = "ok: v6\nnote: rule m -> n allow changes nothing: already allowed by deps.default of grp/brickie.toml\n";
    assert_eq!(text, want);
    let args = [
        "rule", "add", "n", "m", "allow", "--in", "grp", "--base", "6",
    ];
    let out = run(d, &[&args[..], &["--format", "json"]].concat());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["notes"][0].as_str().unwrap().contains("changes nothing"),
        "{v}"
    );
    // Under `default = "deny"` the same rule changes the level: no note.
    ok(d, 7, &["deps", "set", "--default", "deny", "--in", "grp"]);
    ok(d, 8, &["rule", "rm", "m", "n", "--in", "grp"]);
    let out = run(
        d,
        &[
            "rule", "add", "m", "n", "allow", "--in", "grp", "--base", "9",
        ],
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text, "ok: v10\n");
}
