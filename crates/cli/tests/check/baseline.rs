//! Check tests: `--save-baseline`, `--new`, `--format short` and `--measure`.

use super::support::*;
use serde_json::{Value, json};

const RULES: &str = r#"
[deps]
default = "deny"
[[deps.rule]]
from = "*"
to = "port"
action = "allow"
[[deps.rule]]
from = "app"
to = "*"
action = "allow"
[[deps.rule]]
from = "engine"
to = "adapter"
action = "allow"
[[complexity]]
cognitive = 10
"#;

fn cache_path(w: &World) -> PathBuf {
    w.dir.path().join(".brickie/cache.json")
}

use std::path::PathBuf;

fn rewrite_cache(w: &World) {
    std::fs::write(
        cache_path(w),
        serde_json::to_string_pretty(&w.cache).unwrap(),
    )
    .unwrap();
}

fn cfg(w: &World) -> PathBuf {
    w.save(&with_bricks(RULES))
}

fn saved(w: &World) -> PathBuf {
    let c = cfg(w);
    let o = run(&c, &["--save-baseline", "--format", "json"]);
    assert_eq!(o.code, 1, "{}{}", o.stdout, o.stderr);
    c
}

fn msgs(o: &Out) -> Vec<String> {
    let ds = o.json["diagnostics"].as_array().unwrap();
    ds.iter()
        .map(|d| d["message"].as_str().unwrap().into())
        .collect()
}

#[test]
fn save_writes_the_file_and_new_hides_everything() {
    let w = world(|_| {});
    let c = saved(&w);
    let b: Value = serde_json::from_slice(
        &std::fs::read(w.dir.path().join(".brickie/baseline.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(b["version"], 1);
    assert!(b["rule_files"]["brickie.toml"].is_string());
    assert!(b["rule_files"].get("brickie.review.toml").is_some());
    let f = b["findings"].as_object().unwrap();
    assert!(
        f.values()
            .any(|v| v["code"] == "complexity/cognitive" && v["weight"] == 20)
    );
    let o = run(&c, &["--new", "--format", "json"]);
    assert_eq!(o.code, 0, "{}", o.stdout);
    assert_eq!(
        o.json["baseline"]["errors"].as_u64().unwrap() as usize,
        f.len()
    );
    let t = run(&c, &["--new"]);
    assert!(t.stdout.contains("baseline: "), "{}", t.stdout);
    assert!(
        t.stdout.contains(" errors, 0 warnings hidden"),
        "{}",
        t.stdout
    );
}

#[test]
fn new_without_baseline_or_with_save_fails() {
    let w = world(|_| {});
    let c = cfg(&w);
    let o = run(&c, &["--new"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("--save-baseline"), "{}", o.stderr);
    assert_eq!(run(&c, &["--new", "--save-baseline"]).code, 2);
}

#[test]
fn new_dependency_is_shown() {
    let mut w = world(|_| {});
    saved(&w);
    w.cache["files"]["crates/port/src/t.rs"]["edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("port::t", "app::z", "uses", "prod", 4));
    rewrite_cache(&w);
    let o = run(
        &w.dir.path().join("brickie.toml"),
        &["--new", "--format", "json"],
    );
    assert_eq!(o.code, 1);
    assert_eq!(codes(&o, "deps/").len(), 1, "{}", o.stdout);
    assert_eq!(o.json["diagnostics"].as_array().unwrap().len(), 1);
}

#[test]
fn grown_value_is_shown_worse_and_shrunk_is_hidden() {
    let mut w = world(|_| {});
    let c = saved(&w);
    let busy = |w: &mut World, cog: u32| {
        w.cache["files"]["crates/eng/src/x.rs"]["functions"][0]["cognitive"] = json!(cog);
        rewrite_cache(w);
    };
    busy(&mut w, 22);
    let o = run(&c, &["--new", "--format", "json"]);
    assert_eq!(o.code, 1, "{}", o.stdout);
    let d = &o.json["diagnostics"];
    assert_eq!(d.as_array().unwrap().len(), 1, "{}", o.stdout);
    assert_eq!(
        d[0]["notes"][0]["text"],
        "worse than the baseline: 20 -> 22"
    );
    busy(&mut w, 12);
    let o = run(&c, &["--new", "--format", "json"]);
    assert_eq!(o.code, 0, "{}", o.stdout);
    assert!(msgs(&o).is_empty());
}

#[test]
fn changed_rules_fail_the_gate() {
    let w = world(|_| {});
    saved(&w);
    w.write("brickie.toml", &(with_bricks(RULES) + "\n# edited\n"));
    let o = run(
        &w.dir.path().join("brickie.toml"),
        &["--new", "--format", "json"],
    );
    assert_eq!(o.code, 1);
    let d = codes(&o, "baseline/rules-changed");
    assert_eq!(d.len(), 1, "{}", o.stdout);
    assert!(d[0]["message"].as_str().unwrap().contains("brickie.toml"));
    assert!(d[0]["span"].is_null());
}

#[test]
fn a_new_review_file_fails_the_gate() {
    let w = world(|_| {});
    saved(&w);
    w.write(
        "brickie.review.toml",
        "[[accept]]\nid = \"deadbeef\"\nreason = \"x\"\n",
    );
    let o = run(
        &w.dir.path().join("brickie.toml"),
        &["--new", "--format", "json"],
    );
    assert_eq!(o.code, 1);
    let d = codes(&o, "baseline/rules-changed");
    assert_eq!(
        d[0]["message"],
        "rule files changed since the baseline: brickie.review.toml"
    );
}

#[test]
fn short_format_lines() {
    let w = world(|_| {});
    let o = run(&cfg(&w), &["--format", "short"]);
    assert_eq!(o.code, 1);
    let line = o
        .stdout
        .lines()
        .find(|l| l.starts_with("error[complexity/cognitive] "))
        .unwrap_or_else(|| panic!("{}", o.stdout));
    let mut p = line.splitn(3, ' ');
    p.next();
    let id = p.next().unwrap();
    assert_eq!(id.len(), 8);
    let rest = p.next().unwrap();
    assert!(rest.starts_with("crates/eng/src/x.rs:5"), "{rest}");
    assert!(
        rest.contains(": `busy` has cognitive 20 (limit 10)"),
        "{rest}"
    );
    assert!(
        o.stdout.lines().last().unwrap().starts_with("error: "),
        "{}",
        o.stdout
    );
    assert!(!o.stdout.contains("-->"));
}

#[test]
fn measure_flag_parses_and_skips_a_fresh_cache() {
    let w = world(|_| {});
    let c = cfg(&w);
    let o = run(&c, &["--measure", "--format", "json"]);
    assert_eq!(o.code, 1, "{}{}", o.stdout, o.stderr);
    assert!(!o.stderr.contains("wrote"), "{}", o.stderr);
}
