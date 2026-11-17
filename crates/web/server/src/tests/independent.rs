//! Independent libs in the viewer: inbound edges need no link and are hidden (and counted);
//! every outbound edge is drawn, and hidden edges add no score.

use super::*;

const ROOT: &str = r#"
[[brick]]
name = "lib"
members = ["lib"]
tags = ["lib"]
independent = true

[[brick]]
name = "app"
members = ["app"]
tags = ["app"]

[[brick]]
name = "other"
members = ["other"]
tags = ["app"]

[[rule]]
from = "*"
to = "*"
reason = "a flat workspace"
"#;

fn edge(from: &str, to: &str, line: u32) -> Value {
    json!({"from": format!("{from}::m"), "to": format!("{to}::m"), "kind": "uses",
           "scope": "prod", "line": line, "col": 1, "end_col": 5, "symbol": "x", "written": "x"})
}

fn site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("ws");
    let crates = [
        ("lib", vec![edge("lib", "app", 2)]),
        ("app", vec![edge("app", "lib", 2)]),
        ("other", vec![edge("other", "lib", 2)]),
    ];
    let (mut files, mut modules, mut list) = (serde_json::Map::new(), vec![], vec![]);
    for (pkg, edges) in crates {
        let (d, path) = (format!("crates/{pkg}"), format!("crates/{pkg}/src/m.rs"));
        std::fs::create_dir_all(ws.join(&d).join("src")).unwrap();
        std::fs::write(ws.join(&path), "// m\n").unwrap();
        let sha = brickie_facts::sha256_hex(b"// m\n");
        files.insert(
            path.clone(),
            json!({"sha256": sha, "compiled": true, "size": 5, "lines": 1, "scope": "prod",
                   "module": format!("{pkg}::m"), "functions": [], "edges": edges, "items": []}),
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
    std::fs::create_dir_all(dir.path().join(".brickie")).unwrap();
    std::fs::write(dir.path().join(".brickie/cache.json"), cache.to_string()).unwrap();
    Site { dir }
}

fn has_edge(g: &Value, from: &str, to: &str) -> bool {
    let ix = |n: &str| {
        g["bricks"]
            .as_array()
            .unwrap()
            .iter()
            .position(|b| b["name"] == n)
    };
    let (f, t) = (ix(from).unwrap(), ix(to).unwrap());
    g["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["from"] == f && e["to"] == t)
}

#[tokio::test]
async fn clean_inbound_hidden_violations_and_outbound_drawn() {
    let (st, body) = site().get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let g = &v[0]["graph"];
    let lib = &g["bricks"][0];
    assert_eq!(lib["independent"], true);
    assert_eq!(g["bricks"][1]["independent"], false);
    // app -> lib and other -> lib need no link: hidden, but counted for the inspector.
    assert!(!has_edge(g, "app", "lib") && !has_edge(g, "other", "lib"));
    assert_eq!(lib["hidden_in"], 2);
    // lib -> app is an independent error: drawn.
    assert!(has_edge(g, "lib", "app"));
}

#[tokio::test]
async fn hidden_edges_do_not_count_toward_scores() {
    let (_, body) = site().get("/api/graph").await;
    let v: Vec<brickie_web_api::LevelView> = serde_json::from_str(&body).unwrap();
    let g = &v[0].graph;
    let scores = brickie_web_api::brick_scores(g, &[]);
    // Nothing is drawn into lib; the hidden edges add no in-edge.
    assert_eq!(scores[0].edges_in, 0);
    assert_eq!(scores[0].edges_out, 1);
}
