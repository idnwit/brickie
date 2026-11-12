//! `brickie measure` on `fixtures/measure`: edge kinds, test scope, complexity numbers, crate
//! dirs, and that the measured workspace is left untouched. Needs `rust-analyzer` on PATH.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_brickie");

const CONFIG: &str = r#"
[[brick]]
name = "base"
members = ["mx-base"]
tags = ["core"]

[[brick]]
name = "engine"
members = ["mx-app::engine", "mx-app::model"]

[[brick]]
name = "app"
members = ["mx-app"]
"#;

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let to = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}

fn have_rust_analyzer() -> bool {
    Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

struct Measured {
    cache: Value,
    ws: PathBuf,
    _tmp: tempfile::TempDir,
}

fn measure() -> Option<Measured> {
    if !have_rust_analyzer() {
        eprintln!("skipping: rust-analyzer not on PATH");
        return None;
    }
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("ws");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/measure"),
        &ws,
    );
    let cfg = tmp.path().join("brickie.toml");
    std::fs::write(&cfg, CONFIG).unwrap();
    let out = Command::new(BIN)
        .args(["measure", "--config"])
        .arg(&cfg)
        .arg("--workspace")
        .arg(&ws)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let cache = std::fs::read(tmp.path().join(".brickie/cache.json")).unwrap();
    Some(Measured {
        cache: serde_json::from_slice(&cache).unwrap(),
        ws,
        _tmp: tmp,
    })
}

/// Module-level edge derived from the per-file edges (the cache stores no aggregate).
fn edge(c: &Value, from: &str, to: &str, kind: &str, scope: &str) -> Option<Value> {
    let mut evidence = Vec::new();
    for (path, f) in c["files"].as_object().unwrap() {
        for e in f["edges"].as_array().unwrap() {
            if e["from"] == from && e["to"] == to && e["kind"] == kind && e["scope"] == scope {
                evidence.push(serde_json::json!({
                    "file": path, "line": e["line"], "symbol": e["symbol"], "written": e["written"]
                }));
            }
        }
    }
    (!evidence.is_empty()).then(|| serde_json::json!({"evidence": evidence}))
}

fn function<'a>(c: &'a Value, file: &str, qname: &str) -> &'a Value {
    c["files"][file]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["qname"] == qname)
        .unwrap_or_else(|| panic!("no function {qname} in {file}"))
}

#[test]
fn measure_fixture() {
    let Some(m) = measure() else { return };
    let c = &m.cache;
    assert_eq!(c["schema_version"], 6);
    // A file no `mod` declares is recorded, hashed, not compiled, without functions or edges.
    let orphan = &c["files"]["crates/app/src/orphan.rs"];
    assert_eq!(orphan["compiled"], false);
    assert_eq!(orphan["sha256"].as_str().unwrap().len(), 64);
    assert!(orphan["functions"].as_array().unwrap().is_empty());
    assert!(orphan["edges"].as_array().unwrap().is_empty());
    assert_eq!(c["files"]["crates/app/src/api.rs"]["compiled"], true);
    assert!(c.get("edges").is_none());
    assert!(
        c["files"]["crates/app/src/api.rs"]["sha256"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );

    // Edge kinds.
    let uses = edge(c, "mx-app::engine", "mx-base::util", "uses", "prod").unwrap();
    assert!(uses["evidence"].as_array().unwrap().iter().any(|e| {
        e["file"] == "crates/app/src/engine.rs" && e["line"] == 6 && e["symbol"] == "util::double"
    }));
    let imp = edge(c, "mx-app::model", "mx-base::shape", "implements", "prod").unwrap();
    assert_eq!(imp["evidence"][0]["line"], 4);
    assert!(edge(c, "mx-app::api", "mx-app::engine", "reexports", "prod").is_some());
    // A trait impl is an `implements` edge only, not also a `uses` edge.
    assert!(edge(c, "mx-app::model", "mx-base::shape", "uses", "prod").is_none());

    // Re-export: `to` is the defining module (`shape`, not the crate root that re-exports),
    // `written` is the path at the site, and each site yields exactly one kind.
    let at = |kind: &str, line: u64| -> Option<Value> {
        c["files"]["crates/app/src/facade.rs"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["kind"] == kind && e["line"] == line)
            .cloned()
    };
    let private_use = at("uses", 1).unwrap();
    assert_eq!(private_use["to"], "mx-base::shape");
    assert_eq!(private_use["written"], "mx_base::unit");
    let pub_use = at("reexports", 2).unwrap();
    assert_eq!(pub_use["to"], "mx-base::shape");
    assert_eq!(pub_use["written"], "mx_base::unit");
    assert!(at("uses", 2).is_none() && at("reexports", 1).is_none());
    assert_eq!(at("uses", 5).unwrap()["written"], "mx_base::unit");
    let plain = c["files"]["crates/app/src/facade.rs"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["line"] == 5 && e["written"] == "unit")
        .unwrap();
    assert_eq!(plain["kind"], "uses");
    // The façade's own `pub use shape::unit` is a reexports edge root -> shape, written `shape::unit`.
    let root = c["files"]["crates/base/src/lib.rs"]["edges"]
        .as_array()
        .unwrap();
    assert!(
        root.iter()
            .any(|e| e["kind"] == "reexports" && e["written"] == "shape::unit")
    );

    // Test scope: inline cfg(test) module, `mod tests;` file, and the tests/ directory.
    assert!(edge(c, "mx-app::engine::tests", "mx-base::shape", "uses", "test").is_some());
    assert!(edge(c, "mx-app::tests", "mx-base::util", "uses", "test").is_some());
    assert!(
        edge(c, "mx-app/test/it", "mx-app::engine", "uses", "test").is_some(),
        "tests/it.rs entry: {}",
        c["files"]["crates/app/tests/it.rs"]
    );
    assert!(edge(c, "mx-app::engine::tests", "mx-base::shape", "uses", "prod").is_none());

    // Per-file edges carry the same evidence.
    let fe = c["files"]["crates/app/src/api.rs"]["edges"]
        .as_array()
        .unwrap();
    assert!(
        fe.iter()
            .any(|e| e["kind"] == "reexports" && e["line"] == 2)
    );
    assert!(fe.iter().all(|e| e["col"].as_u64().unwrap() >= 1
        && e["end_col"].as_u64().unwrap() > e["col"].as_u64().unwrap()));

    // No brick mapping in the cache (check resolves it); crates record their package dir.
    assert!(c.get("bricks").is_none());
    let base = c["crates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == "mx-base")
        .unwrap();
    assert_eq!(base["dir"], "crates/base");

    // The measured workspace is untouched (no Cargo.lock left behind).
    assert!(!m.ws.join("Cargo.lock").exists());
}

#[test]
fn measure_complexity() {
    let Some(m) = measure() else { return };
    let c = &m.cache;
    let file = "crates/app/src/complexity.rs";
    // (qname, cognitive, cyclomatic, nesting, paths, params)
    let expect = [
        ("straight", 0, 1, 0, 1, 2),
        // `&self` is not counted.
        ("Counter::add", 0, 1, 0, 1, 1),
        // if +1, else if +1, else +1.
        ("branches", 3, 3, 1, 3, 1),
        // for +1, if at nesting 1 +2, `&&` then `||` +2.
        ("nested", 5, 5, 2, 5, 3),
        // if +1, else +1, recursive call +1.
        ("fact", 3, 2, 1, 2, 1),
        // match +1, inner if at nesting 1 +2, else +1; paths 1 + 2 + 1.
        ("matcher", 4, 4, 2, 4, 1),
        // flat lookup table: one decision.
        ("lookup", 1, 3, 1, 1, 1),
        // let-else +1.
        ("parse", 1, 2, 1, 2, 1),
    ];
    for (name, cog, cyc, nest, paths, params) in expect {
        let f = function(c, file, name);
        let got = (
            f["cognitive"].as_u64().unwrap(),
            f["cyclomatic"].as_u64().unwrap(),
            f["nesting"].as_u64().unwrap(),
            f["paths"].as_u64().unwrap(),
            f["params"].as_u64().unwrap(),
        );
        assert_eq!(got, (cog, cyc, nest, paths, params), "{name}");
        assert_eq!(f["scope"], "prod", "{name}");
    }
    assert_eq!(function(c, file, "straight")["lines"], 3);
    assert_eq!(function(c, file, "straight")["line"], 1);

    // Test gating: cfg(not(test)) is prod; test-util and any(test, ..) are test.
    assert_eq!(function(c, file, "prod_only")["scope"], "prod");
    assert_eq!(function(c, file, "test_util")["scope"], "test");
    assert_eq!(function(c, file, "maybe_test")["scope"], "test");
    assert_eq!(
        function(c, "crates/app/src/engine.rs", "uses_shape")["scope"],
        "test"
    );
    assert_eq!(
        function(c, "crates/app/src/tests.rs", "helper")["scope"],
        "test"
    );
    assert_eq!(
        function(c, "crates/app/src/model.rs", "Square::area")["module"],
        "mx-app::model"
    );
}

/// A measure while another holds the cache lock fails at once, naming the holder; the cache is
/// not touched. The lock holder here is this test process (alive).
#[test]
fn measure_refuses_while_the_cache_is_locked() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("brickie.toml"), CONFIG).unwrap();
    let dir = tmp.path().join(".brickie");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cache.lock"), std::process::id().to_string()).unwrap();
    let out = Command::new(BIN)
        .args(["measure", "--config"])
        .arg(tmp.path().join("brickie.toml"))
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(err.contains("another measure"), "{err}");
    assert!(!dir.join("cache.json").exists());
    assert!(dir.join("cache.lock").exists(), "the holder's lock stays");
}
