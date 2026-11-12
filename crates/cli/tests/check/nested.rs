//! Check tests: a two-level brick tree. `web` is a folder of three crates with its own
//! `brickie.toml`; findings carry their level and point at the file that owns the rule.

use super::support::*;
use serde_json::{Value, json};

const ROOT: &str = r#"
[[brick]]
name = "web"
path = "ws/crates/web"
tags = ["app"]

[[brick]]
name = "core"
members = ["core"]
tags = ["core"]

[deps]
default = "deny"
cycles = "deny"

[[deps.rule]]
from = "tag:app"
to = "tag:core"
action = "allow"
"#;

const WEB: &str = r#"
exports = ["api", { brick = "ui", scope = "test" }]

[[brick]]
name = "api"
members = ["web-api"]

[[brick]]
name = "ui"
members = ["web-ui"]

[[brick]]
name = "shared"
members = ["web-shared"]

[deps]
default = "deny"

[[deps.rule]]
from = "*"
to = "shared"
action = "allow"

[[deps.rule]]
from = "ui"
to = "api"
action = "allow"

[[complexity]]
bricks = "ui"
cognitive = 10
"#;

/// (package, dir under ws/, edges from its module `<package>::m`, functions).
fn tree() -> World {
    let crates: [(&str, &str, Vec<Value>, Vec<Value>); 4] = [
        (
            "core",
            "crates/core",
            vec![
                edge("core::m", "web-api::m", "uses", "test", 2),
                edge("core::m", "web-ui::m", "uses", "test", 3),
                edge("core::m", "web-shared::m", "uses", "test", 4),
            ],
            vec![],
        ),
        (
            "web-api",
            "crates/web/api",
            vec![edge("web-api::m", "core::m", "uses", "prod", 2)],
            vec![],
        ),
        (
            "web-ui",
            "crates/web/ui",
            vec![
                edge("web-ui::m", "web-api::m", "uses", "prod", 2),
                edge("web-ui::m", "web-shared::m", "uses", "prod", 3),
            ],
            vec![func("busy", "web-ui::m", 5, 12, "prod")],
        ),
        (
            "web-shared",
            "crates/web/shared",
            vec![edge("web-shared::m", "web-ui::m", "uses", "prod", 2)],
            vec![],
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let w = World {
        dir,
        cache: Value::Null,
    };
    let (mut files, mut modules, mut list) = (serde_json::Map::new(), vec![], vec![]);
    for (pkg, d, edges, fns) in crates {
        let path = format!("{d}/src/m.rs");
        let text = source(&path);
        w.write(&format!("ws/{path}"), &text);
        files.insert(
            path.clone(),
            json!({"sha256": sha(&text), "compiled": true, "size": text.len(), "lines": 10,
                   "scope": "prod", "module": format!("{pkg}::m"), "functions": fns, "edges": edges, "items": []}),
        );
        modules.push(json!({"id": format!("{pkg}::m"), "crate": pkg, "file": path, "line": 1}));
        list.push(
            json!({"id": pkg, "package": pkg, "version": "0.1.0", "target": pkg,
                         "kind": "lib", "root": format!("{d}/src/lib.rs"), "dir": d}),
        );
    }
    let cache = json!({
        "schema_version": 6, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": w.ws().display().to_string(), "git": null,
        "crates": list, "modules": modules, "files": files,
    });
    World { cache, ..w }
}

fn find<'a>(o: &'a Out, code: &str, level: &str) -> Vec<&'a Value> {
    codes(o, code)
        .into_iter()
        .filter(|d| d["brick_level"] == level)
        .collect()
}

#[test]
fn findings_carry_level_and_owning_file() {
    let w = tree();
    w.write("ws/crates/web/brickie.toml", WEB);
    let o = check(&w, ROOT);
    assert_eq!(o.code, 1, "{}", o.stderr);
    // Root: core -> web is denied by default (test edges are checked too).
    let root = find(&o, "deps/default", "root");
    assert_eq!(root.len(), 3, "{}", o.stdout);
    assert!(root.iter().all(|d| d["bricks"] == json!(["core", "web"])));
    assert!(
        root[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .ends_with("/brickie.toml")
    );
    // web: shared -> ui has no rule; the finding points at web's own file.
    let web = find(&o, "deps/default", "root/web");
    assert_eq!(web.len(), 1, "{}", o.stdout);
    assert_eq!(web[0]["bricks"], json!(["shared", "ui"]));
    assert!(
        web[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .ends_with("ws/crates/web/brickie.toml")
    );
    // Visibility: shared is private; ui is exported to test edges only.
    let private = find(&o, "deps/private", "root/web");
    assert_eq!(private.len(), 1, "{}", o.stdout);
    assert_eq!(private[0]["bricks"], json!(["shared"]));
    assert_eq!(private[0]["span"]["line"], 4);
    // Root cycles are on prod edges only: core -> web is test-only here.
    assert_eq!(
        find(&o, "deps/cycle", "root").len(),
        0,
        "test edges form no cycle"
    );
    // Complexity from web's file applies to ui.
    let cx = find(&o, "complexity/cognitive", "root/web");
    assert_eq!(cx.len(), 1);
    assert!(
        cx[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .contains("ws/crates/web/brickie.toml:")
    );
    assert!(codes(&o, "bricks/unmapped").is_empty());
}

#[test]
fn partition_and_missing_config_exit_2() {
    let w = tree();
    let o = check(&w, ROOT);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("config/missing"), "{}", o.stderr);
    let cases = [
        (
            WEB.replace("members = [\"web-shared\"]", "members = []"),
            "config/unassigned",
        ),
        (
            WEB.replace(
                "members = [\"web-shared\"]",
                "members = [\"web-shared\", \"core\"]",
            ),
            "config/member-outside-parent",
        ),
        (
            WEB.replace(
                "members = [\"web-ui\"]",
                "members = [\"web-ui\", \"web-api\"]",
            ),
            "config/overlap",
        ),
        (
            WEB.replace("exports = [\"api\"", "exports = [\"ghost\""),
            "config/unknown-export",
        ),
        (
            WEB.replace("to = \"shared\"", "to = \"core\""),
            "unknown brick 'core'",
        ),
    ];
    for (web, needle) in cases {
        w.write("ws/crates/web/brickie.toml", &web);
        let o = check(&w, ROOT);
        assert_eq!(o.code, 2, "{needle}");
        assert!(o.stderr.contains(needle), "{needle} not in {}", o.stderr);
    }
}
