//! Drift is what the change introduces; a cache of another workspace is one diagnostic.

use super::*;
use crate::TaskKind;
use serde_json::{Value, json};

fn file(module: &str, edge_to: Option<&str>) -> Value {
    let edges: Vec<Value> = edge_to
        .iter()
        .map(|to| {
            json!({"from": module, "to": to, "kind": "use", "scope": "normal", "line": 1,
                   "col": 1, "end_col": 2, "symbol": "x", "written": "x"})
        })
        .collect();
    json!({"sha256": "0", "compiled": true, "size": 0, "lines": 1, "scope": "normal",
           "module": module, "functions": [], "edges": edges, "items": []})
}

/// Modules pa, pb, pc; `pb -> pa` is denied by the design.
fn cache(ws: &Ws, root: &str) {
    let ids = ["pa", "pb", "pc"];
    let crates: Vec<Value> = ids
        .iter()
        .map(|i| {
            json!({"id": i, "package": i, "version": "0", "target": i, "kind": "lib",
                   "root": format!("{i}/lib.rs"), "dir": i})
        })
        .collect();
    let modules: Vec<Value> = ids
        .iter()
        .map(|i| json!({"id": i, "crate": i, "file": format!("{i}/lib.rs"), "line": 1}))
        .collect();
    let files = json!({
        "pa/lib.rs": file("pa", None),
        "pb/lib.rs": file("pb", Some("pa")),
        "pc/lib.rs": file("pc", Some("pa")),
    });
    let cache = json!({"schema_version": 6, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": root, "git": null, "crates": crates, "modules": modules,
        "files": files});
    ws.write(".brickie/cache.json", &cache.to_string());
}

fn drift(c: &Change) -> Vec<String> {
    let todo = c.todo().unwrap();
    todo.iter()
        .filter(|t| t.kind == TaskKind::Drift)
        .map(|t| t.text.clone())
        .collect()
}

#[test]
fn drift_lists_only_findings_the_change_introduces() {
    let ws = Ws::new();
    cache(&ws, &ws.dir.path().display().to_string());
    let c = ws.open();
    let before = c.todo().unwrap();
    assert!(before.is_empty(), "base findings are not drift: {before:?}");
    c.record(H, add("c", &["pc"]), None).unwrap();
    let d = drift(&c);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("`c` uses `a`"), "{d:?}");
}

#[test]
fn a_cache_of_another_workspace_is_one_diagnostic() {
    let ws = Ws::new();
    cache(&ws, "/somewhere/else/entirely");
    let c = ws.open();
    c.record(H, add("c", &["pc"]), None).unwrap();
    let todo = c.todo().unwrap();
    assert_eq!(todo.len(), 1, "{todo:?}");
    assert_eq!(todo[0].kind, TaskKind::Config);
    assert!(todo[0].text.contains("cache does not match"), "{todo:?}");
}

#[test]
fn a_cache_matching_no_brick_is_one_diagnostic() {
    let ws = Ws::new();
    cache(&ws, &ws.dir.path().display().to_string());
    ws.write(
        "brickie.toml",
        &ROOT.replace("\"pa\"", "\"zz\"").replace("\"pb\"", "\"yy\""),
    );
    let c = ws.open();
    let todo = c.todo().unwrap();
    assert_eq!(todo.len(), 1, "{todo:?}");
    assert!(todo[0].text.contains("cache does not match"), "{todo:?}");
}
