//! Tree resolution and partition checks on a two-level fixture (configs on disk, cache facts
//! built in code). `fixture` is shared with the rule tests.

use super::{Config, Resolved, load, resolve};
use brickie_facts::Cache;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) const ROOT: &str = r#"
[[brick]]
name = "web"
path = "crates/web"
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

[[complexity]]
cognitive = 20
"#;

pub(crate) const WEB: &str = r#"
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
cycles = "deny"

[[deps.rule]]
from = "*"
to = "shared"
action = "allow"

[[deps.rule]]
from = "ui"
to = "api"
action = "allow"

[[deps.rule]]
from = "api"
to = "ui"
action = "allow"

[[complexity]]
bricks = "ui"
cognitive = 10
"#;

/// Crates (package, dir), one module `<package>::m` each.
const CRATES: [(&str, &str); 4] = [
    ("core", "crates/core"),
    ("web-api", "crates/web/api"),
    ("web-ui", "crates/web/ui"),
    ("web-shared", "crates/web/shared"),
];

pub(crate) fn edge(from: &str, to: &str, scope: &str) -> Value {
    json!({"from": format!("{from}::m"), "to": format!("{to}::m"), "kind": "uses",
           "scope": scope, "line": 2, "col": 1, "end_col": 5, "symbol": "s", "written": "w"})
}

pub(crate) fn func(name: &str, module: &str, cognitive: u32) -> Value {
    json!({"name": name, "qname": name, "module": format!("{module}::m"), "line": 1,
           "end_line": 2, "lines": 2, "cognitive": cognitive, "cyclomatic": 1, "nesting": 0,
           "paths": 1, "params": 0, "scope": "prod"})
}

/// A cache over `CRATES` with the given (from package, edges, functions) per module file.
pub(crate) fn cache(ws: &Path, per: &[(&str, Vec<Value>, Vec<Value>)]) -> Cache {
    let mut files = serde_json::Map::new();
    for (pkg, edges, fns) in per {
        let dir = CRATES.iter().find(|c| c.0 == *pkg).unwrap().1;
        files.insert(
            format!("{dir}/src/m.rs"),
            json!({"sha256": "", "compiled": true, "size": 0, "lines": 2, "scope": "prod",
                   "module": format!("{pkg}::m"), "functions": fns, "edges": edges, "items": []}),
        );
    }
    let crates: Vec<Value> = CRATES
        .iter()
        .map(|(p, d)| {
            json!({"id": p, "package": p, "version": "0", "target": p, "kind": "lib",
                   "root": format!("{d}/src/lib.rs"), "dir": d})
        })
        .collect();
    let modules: Vec<Value> = CRATES
        .iter()
        .map(|(p, d)| json!({"id": format!("{p}::m"), "crate": p, "file": format!("{d}/src/m.rs"), "line": 1}))
        .collect();
    serde_json::from_value(json!({
        "schema_version": brickie_facts::SCHEMA_VERSION, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": ws.display().to_string(), "git": null,
        "crates": crates, "modules": modules, "files": files,
    }))
    .unwrap()
}

/// Writes the root and web configs into a temp dir, which is also the workspace root.
pub(crate) fn fixture(root: &str, web: &str) -> (tempfile::TempDir, Config) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("crates/web")).unwrap();
    std::fs::write(dir.path().join("brickie.toml"), root).unwrap();
    std::fs::write(dir.path().join("crates/web/brickie.toml"), web).unwrap();
    let cfg = load(&dir.path().join("brickie.toml")).unwrap();
    (dir, cfg)
}

pub(crate) fn resolved(dir: &Path, cfg: &Config) -> anyhow::Result<Resolved> {
    let ws = dir.canonicalize().unwrap();
    let c = cache(&ws, &[]);
    resolve(cfg, &c.crates, &c.modules, &ws)
}

#[test]
fn modules_resolve_to_leaf_paths() {
    let (dir, cfg) = fixture(ROOT, WEB);
    let r = resolved(dir.path(), &cfg).unwrap();
    assert_eq!(r.path("core::m").unwrap(), ["core"]);
    assert_eq!(r.path("web-api::m").unwrap(), ["web", "api"]);
    assert_eq!(r.path("web-shared::m").unwrap()[0], "web");
}

#[test]
fn partition_errors() {
    let err = |root: &str, web: &str| {
        let (dir, cfg) = fixture(root, web);
        format!("{:#}", resolved(dir.path(), &cfg).unwrap_err())
    };
    // A web module in no child.
    let web = WEB.replace("members = [\"web-shared\"]", "members = []");
    assert!(err(ROOT, &web).contains("config/unassigned"));
    // A child member reaching a crate outside `crates/web`.
    let web = WEB.replace(
        "members = [\"web-shared\"]",
        "members = [\"web-shared\", \"core\"]",
    );
    assert!(err(ROOT, &web).contains("config/member-outside-parent"));
    // Two children with equal specificity.
    let web = WEB.replace(
        "members = [\"web-ui\"]",
        "members = [\"web-ui\", \"web-api\"]",
    );
    assert!(err(ROOT, &web).contains("config/overlap"));
}

#[test]
fn longer_path_member_beats_shorter() {
    let web = WEB
        .replace("members = [\"web-api\"]", "members = [\"path:api\"]")
        .replace("members = [\"web-ui\"]", "members = [\"path:./\"]")
        .replace("members = [\"web-shared\"]", "members = [\"path:shared/\"]");
    let (dir, cfg) = fixture(ROOT, &web);
    let r = resolved(dir.path(), &cfg).unwrap();
    assert_eq!(r.path("web-api::m").unwrap(), ["web", "api"]);
    assert_eq!(r.path("web-ui::m").unwrap(), ["web", "ui"]);
    assert_eq!(r.path("web-shared::m").unwrap(), ["web", "shared"]);
}
