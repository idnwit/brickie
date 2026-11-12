//! The graph data per level (`/api/graph`): a two-level tree where `web` is a composite that
//! exports only `api`. Checks children, outside neighbours lifted to the nearest enclosing
//! level, aggregated counts, findings on lifted edges and the private edge stopping at the wall.

use super::*;

const ROOT: &str = r#"
[[brick]]
name = "web"
path = "ws/crates/web"
tags = ["app"]

[[brick]]
name = "core"
members = ["core"]
tags = ["core"]

[[brick]]
name = "client"
members = ["client"]
tags = ["app"]

[deps]
default = "deny"

[[deps.rule]]
from = "tag:app"
to = "tag:core"
action = "allow"

[[deps.rule]]
from = "client"
to = "web"
action = "allow"
"#;

const WEB: &str = r#"
exports = ["api"]

[[brick]]
name = "api"
members = ["web-api"]

[[brick]]
name = "ui"
members = ["web-ui"]

[deps]
default = "allow"
"#;

fn edge(from: &str, to: &str, line: u32) -> Value {
    json!({"from": format!("{from}::m"), "to": format!("{to}::m"), "kind": "uses",
           "scope": "prod", "line": line, "col": 1, "end_col": 5, "symbol": "x", "written": "x"})
}

/// core -> client is denied at the root; client -> web/ui reaches a child `web` hides.
fn site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let crates: [(&str, &str, Vec<Value>); 4] = [
        (
            "web-api",
            "crates/web/api",
            vec![edge("web-api", "core", 2), edge("web-api", "core", 3)],
        ),
        (
            "web-ui",
            "crates/web/ui",
            vec![edge("web-ui", "web-api", 2)],
        ),
        ("core", "crates/core", vec![edge("core", "client", 2)]),
        (
            "client",
            "crates/client",
            vec![edge("client", "web-ui", 2), edge("client", "web-api", 3)],
        ),
    ];
    let ws = dir.path().join("ws");
    let (mut files, mut modules, mut list) = (serde_json::Map::new(), vec![], vec![]);
    for (pkg, d, edges) in crates {
        let path = format!("{d}/src/m.rs");
        let text = "// m\n";
        std::fs::create_dir_all(ws.join(d).join("src")).unwrap();
        std::fs::write(ws.join(&path), text).unwrap();
        let sha = brickie_facts::sha256_hex(text.as_bytes());
        files.insert(
            path.clone(),
            json!({"sha256": sha, "compiled": true, "size": text.len(), "lines": 1,
                   "scope": "prod", "module": format!("{pkg}::m"), "functions": [], "edges": edges,
                   "items": []}),
        );
        modules.push(json!({"id": format!("{pkg}::m"), "crate": pkg, "file": path, "line": 1}));
        list.push(
            json!({"id": pkg, "package": pkg, "version": "0.1.0", "target": pkg,
                         "kind": "lib", "root": format!("{d}/src/lib.rs"), "dir": d}),
        );
    }
    let cache = json!({
        "schema_version": brickie_facts::SCHEMA_VERSION, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": ws.display().to_string(), "git": null,
        "crates": list, "modules": modules, "files": files,
    });
    std::fs::write(dir.path().join("brickie.toml"), ROOT).unwrap();
    std::fs::write(ws.join("crates/web/brickie.toml"), WEB).unwrap();
    std::fs::create_dir_all(dir.path().join(".brickie")).unwrap();
    std::fs::write(dir.path().join(".brickie/cache.json"), cache.to_string()).unwrap();
    Site { dir }
}

/// The edge `from -> to` of a level graph, by brick name.
fn edge_of<'a>(g: &'a Value, from: &str, to: &str) -> &'a Value {
    let names: Vec<&str> = g["bricks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    let ix = |n: &str| names.iter().position(|x| *x == n).unwrap();
    let (f, t) = (ix(from), ix(to));
    g["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["from"] == f && e["to"] == t)
        .unwrap_or_else(|| panic!("no edge {from} -> {to} in {g}"))
}

#[tokio::test]
async fn graph_is_aggregated_per_level() {
    let s = site();
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let paths: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["root", "root/web"]);

    // Root: three children, no neighbours; web is a composite exporting api.
    let root = &v[0]["graph"];
    let names: Vec<&str> = root["bricks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["web", "core", "client"]);
    let web = &root["bricks"][0];
    assert_eq!(web["path"], "root/web");
    assert_eq!(web["children"], json!(["api", "ui"]));
    assert_eq!(web["exports"], json!(["api"]));
    assert!(web["file"].as_str().unwrap().ends_with("brickie.toml"));
    assert_eq!(root["bricks"][1]["children"], json!([]));
    // Two module edges with two references in total, lifted to web -> core.
    let wc = edge_of(root, "web", "core");
    assert_eq!(wc["count"], 2);
    assert_eq!(wc["diags"], json!([]));
    assert_eq!(wc["worst"], Value::Null);
    let cc = edge_of(root, "core", "client");
    assert_eq!(cc["diags"].as_array().unwrap().len(), 1);
    assert_eq!(cc["worst"], "error");
    // client -> web carries the private finding of root/web (red here, no wall at the root).
    let cw = edge_of(root, "client", "web");
    assert_eq!(cw["count"], 2);
    assert_eq!(cw["diags"].as_array().unwrap().len(), 1);
    assert_eq!(cw["wall"], false);

    // root/web: children, then outside neighbours (sources row first, targets row last).
    let web = &v[1];
    assert_eq!(web["exports"], json!(["api"]));
    let g = &web["graph"];
    let bricks = g["bricks"].as_array().unwrap();
    let names: Vec<&str> = bricks.iter().map(|b| b["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["api", "ui", "root/client", "root/core"]);
    assert_eq!(bricks[0]["exported"], true);
    assert_eq!(bricks[1]["exported"], false);
    assert!(bricks[2]["ghost"].as_bool().unwrap() && bricks[3]["ghost"].as_bool().unwrap());
    let rows = g["rows"].as_array().unwrap();
    assert_eq!(rows.first().unwrap(), "inbound");
    assert_eq!(rows.last().unwrap(), "outbound");
    assert_eq!(
        g["rows"][bricks[2]["row"].as_u64().unwrap() as usize],
        "inbound"
    );
    assert_eq!(
        g["rows"][bricks[3]["row"].as_u64().unwrap() as usize],
        "outbound"
    );
    // The private edge from outside stops at the wall; the exported one does not.
    let into_ui = edge_of(g, "root/client", "ui");
    assert_eq!(into_ui["wall"], true);
    assert_eq!(into_ui["worst"], "error");
    assert_eq!(edge_of(g, "root/client", "api")["wall"], false);
    assert_eq!(edge_of(g, "api", "root/core")["count"], 2);
    assert_eq!(edge_of(g, "ui", "api")["diags"], json!([]));
    // The same finding id marks the edge at both levels.
    assert_eq!(cw["diags"], into_ui["diags"]);

    // The findings name children of their level.
    let d: Value =
        serde_json::from_str(&s.get("/api/diagnostics?code=deps/private").await.1).unwrap();
    assert_eq!(d[0]["brick_level"], "root/web");
}
