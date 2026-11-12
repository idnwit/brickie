//! Golden JSON of `/api/graph` for the `levels` fixture: pins row order, brick order (children,
//! then the faded outside neighbours), edge order, findings and walls byte for byte.

use super::levels::site;
use super::*;

/// `site()` plus hot functions (a ratio above 1 in `web-api`, one under it in `core`) and a
/// test-only `implements` edge from `web-ui` to `core`.
fn hot_site() -> Site {
    let s = site();
    let root = s.dir.path();
    let cfg = root.join("brickie.toml");
    let mut text = std::fs::read_to_string(&cfg).unwrap();
    text.push_str("\n[[complexity]]\nbricks = \"*\"\ncognitive = 10\n");
    std::fs::write(&cfg, text).unwrap();
    let cache = root.join(".brickie/cache.json");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
    let func = |name: &str, module: &str, line: u32, cog: u32| {
        json!({"name": name, "qname": name, "module": module, "line": line, "end_line": line + 1,
               "lines": 2, "cognitive": cog, "cyclomatic": 1, "nesting": 0, "paths": 1,
               "params": 0, "scope": "prod"})
    };
    let files = v["files"].as_object_mut().unwrap();
    files.get_mut("crates/web/api/src/m.rs").unwrap()["functions"] = json!([
        func("slow", "web-api::m", 1, 30),
        func("slower", "web-api::m", 5, 40),
        func("fine", "web-api::m", 9, 2)
    ]);
    files.get_mut("crates/core/src/m.rs").unwrap()["functions"] =
        json!([func("calm", "core::m", 1, 8)]);
    let ui = files.get_mut("crates/web/ui/src/m.rs").unwrap();
    ui["edges"].as_array_mut().unwrap().push(json!({
        "from": "web-ui::m", "to": "core::m", "kind": "implements", "scope": "test", "line": 3,
        "col": 1, "end_col": 5, "symbol": "x", "written": "x"}));
    std::fs::write(&cache, v.to_string()).unwrap();
    s
}

#[tokio::test]
async fn graph_json_matches_the_golden_snapshot() {
    check(
        site(),
        "levels_golden.json",
        include_str!("levels_golden.json"),
    )
    .await;
}

#[tokio::test]
async fn graph_json_with_heat_and_test_edges_matches_the_golden_snapshot() {
    check(
        hot_site(),
        "levels_hot_golden.json",
        include_str!("levels_hot_golden.json"),
    )
    .await;
}

/// Compare `/api/graph` with the golden `want`; `BRICKIE_BLESS=1` rewrites file `name`.
async fn check(s: Site, name: &str, want: &str) {
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let root = s.dir.path().display().to_string();
    let v: Value = serde_json::from_str(&body.replace(&root, "<site>")).unwrap();
    let got = serde_json::to_string_pretty(&v).unwrap();
    if std::env::var_os("BRICKIE_BLESS").is_some() {
        let p = format!("{}/src/tests/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(p, format!("{got}\n")).unwrap();
    }
    assert_eq!(got.trim_end(), want.trim_end());
}
