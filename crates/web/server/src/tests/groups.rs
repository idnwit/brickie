//! Peer-group server integration tests: `/api/graph` (group node, `group_members`,
//! `internal_edges`, ghost group inside a composite) and `/api/design/edit` (group ops).

use super::*;

const ROOT: &str = r#"
[[group]]
name = "services"
tags = ["domain"]
desc = "Independent domain services"

[[brick]]
name = "auth"
members = ["auth"]
group = "services"

[[brick]]
name = "billing"
members = ["billing"]
group = "services"

[[brick]]
name = "web"
path = "ws/crates/web"
tags = ["app"]

[[link]]
from = "web"
to = "services"
reason = "web calls services"
"#;

const WEB: &str = r#"
[[brick]]
name = "api"
members = ["web-api"]
"#;

fn edge(from: &str, to: &str, line: u32) -> Value {
    json!({"from": format!("{from}::m"), "to": format!("{to}::m"), "kind": "uses",
           "scope": "prod", "line": line, "col": 1, "end_col": 5, "symbol": "x", "written": "x"})
}

fn group_site() -> Site {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("ws");
    // auth -> billing is an intra-group violation; web-api -> auth crosses into root/web as a ghost group.
    let crates: [(&str, &str, Vec<Value>); 3] = [
        ("auth", "crates/auth", vec![edge("auth", "billing", 2)]),
        ("billing", "crates/billing", vec![]),
        (
            "web-api",
            "crates/web/api",
            vec![edge("web-api", "auth", 2)],
        ),
    ];
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

#[tokio::test]
async fn graph_api_serves_group_members_internal_edges_and_ghost_group() {
    let s = group_site();
    let (st, text) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let levels: Vec<brickie_web_api::LevelView> = serde_json::from_str(&text).unwrap();

    // Root level: `services` is a peer group with members [auth, billing] and one intra-group edge.
    let root = &levels[0];
    let grp = root
        .graph
        .bricks
        .iter()
        .find(|b| b.name == "services")
        .expect("services group node");
    assert!(grp.is_group && !grp.ghost);
    let member_names: Vec<&str> = grp.group_members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(member_names, ["auth", "billing"]);
    assert_eq!(grp.internal_edges.len(), 1);
    assert_eq!(
        (
            grp.internal_edges[0].from_member.as_str(),
            grp.internal_edges[0].to_member.as_str()
        ),
        ("auth", "billing")
    );
    assert!(!grp.internal_edges[0].diags.is_empty());

    // Composite `root/web` level: `root/services` appears as a ghost peer group with no member cards.
    let web = levels
        .iter()
        .find(|l| l.path == "root/web")
        .expect("root/web level");
    let ghost = web
        .graph
        .bricks
        .iter()
        .find(|b| b.path == "root/services")
        .expect("ghost services group");
    assert!(ghost.ghost && ghost.is_group);
    assert!(ghost.group_members.is_empty());
}

#[tokio::test]
async fn design_edit_supports_group_ops() {
    let s = group_site();
    let (st, _) = s.get("/api/design").await;
    assert_eq!(st, StatusCode::OK);

    // Add a new group, move `billing` into it, rename the group, then remove it.
    let ops = json!([
        {"op": "add_group", "file": "brickie.toml", "group": {"name": "workers", "tags": ["domain"]}},
        {"op": "set_brick", "file": "brickie.toml", "name": "billing", "group": "workers"},
        {"op": "rename_group", "file": "brickie.toml", "name": "workers", "new_name": "jobs"}
    ]);
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 1, "ops": ops}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let levels: Vec<brickie_web_api::LevelView> =
        serde_json::from_value(r["levels"].clone()).unwrap();
    let jobs = levels[0]
        .graph
        .bricks
        .iter()
        .find(|b| b.name == "jobs")
        .expect("renamed group jobs");
    assert!(jobs.is_group);
    let members: Vec<&str> = jobs.group_members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(members, ["billing"]);

    // Ungroup billing and remove the now-empty group.
    let ops = json!([
        {"op": "set_brick", "file": "brickie.toml", "name": "billing", "group": ""},
        {"op": "remove_group", "file": "brickie.toml", "name": "jobs"}
    ]);
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 2, "ops": ops}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let levels: Vec<brickie_web_api::LevelView> =
        serde_json::from_value(r["levels"].clone()).unwrap();
    assert!(!levels[0].graph.bricks.iter().any(|b| b.name == "jobs"));
    assert!(
        levels[0]
            .graph
            .bricks
            .iter()
            .any(|b| b.name == "billing" && !b.is_group)
    );
}
