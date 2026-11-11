//! Shared helpers: hand-built caches and the `brickie check` runner.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const BIN: &str = env!("CARGO_BIN_EXE_brickie");

pub fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Source of every fixture file: 10 numbered lines, line 2 is a call site.
pub fn source(name: &str) -> String {
    let mut s = format!("// {name}\n    let io = AudioIo::build();\n");
    for i in 3..=10 {
        s += &if i == 5 {
            "pub fn busy() {}\n".to_string()
        } else {
            format!("// line {i}\n")
        };
    }
    s
}

/// Config hash of the brick mapping, computed as `config::hash` does (JSON of the bricks).
pub fn brick_hash(defs: &[(&str, &str, &str)]) -> String {
    let parts: Vec<String> = defs
        .iter()
        .map(|(n, m, t)| format!(r#"{{"name":"{n}","members":["{m}"],"tags":[{t}]}}"#))
        .collect();
    sha(&format!("[{}]", parts.join(",")))
}

pub const BRICKS: &str = r#"
[[brick]]
name = "port"
members = ["port"]
[[brick]]
name = "engine"
members = ["eng"]
tags = ["core"]
[[brick]]
name = "adapter"
members = ["adp"]
tags = ["backend"]
[[brick]]
name = "app"
members = ["app"]
"#;

pub fn defs() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("port", "port", ""),
        ("engine", "eng", "\"core\""),
        ("adapter", "adp", "\"backend\""),
        ("app", "app", ""),
    ]
}

pub fn edge(from: &str, to: &str, kind: &str, scope: &str, line: u32) -> Value {
    json!({"from": from, "to": to, "kind": kind, "scope": scope, "line": line,
           "col": 14, "end_col": 28, "symbol": format!("{to}::item"), "written": "item"})
}

pub fn func(name: &str, module: &str, line: u32, cog: u32, scope: &str) -> Value {
    json!({"name": name, "qname": name, "module": module, "line": line, "end_line": line + 3,
           "lines": 4, "cognitive": cog, "cyclomatic": 2, "nesting": 1, "paths": 3,
           "params": 2, "scope": scope})
}

pub struct World {
    pub dir: tempfile::TempDir,
    pub cache: Value,
}

impl World {
    pub fn ws(&self) -> PathBuf {
        self.dir.path().join("ws")
    }

    pub fn write(&self, rel: &str, text: &str) {
        let p = self.dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    /// Write config text and the cache; returns the config path.
    pub fn save(&self, config: &str) -> PathBuf {
        self.write("brickie.toml", config);
        self.write(
            ".brickie/cache.json",
            &serde_json::to_string_pretty(&self.cache).unwrap(),
        );
        self.dir.path().join("brickie.toml")
    }
}

/// Four bricks (port, engine, adapter, app) plus one unmapped module `misc::q`.
/// Edges: eng->adp, eng->port (implements), adp->port, app->eng, app->adp.
pub fn world(mutate: impl FnOnce(&mut Value)) -> World {
    let dir = tempfile::tempdir().unwrap();
    let mods = [
        (
            "eng",
            "x",
            vec![
                edge("eng::x", "adp::y", "uses", "prod", 2),
                edge("eng::x", "port::t", "implements", "prod", 3),
            ],
            vec![
                func("busy", "eng::x", 5, 20, "prod"),
                func("quiet", "eng::x", 9, 3, "prod"),
                func("busy_test", "eng::x", 7, 30, "test"),
            ],
        ),
        (
            "adp",
            "y",
            vec![edge("adp::y", "port::t", "uses", "prod", 2)],
            vec![],
        ),
        (
            "app",
            "z",
            vec![
                edge("app::z", "eng::x", "uses", "prod", 2),
                edge("app::z", "adp::y", "uses", "prod", 3),
            ],
            vec![],
        ),
        (
            "port",
            "t",
            vec![],
            vec![func("heavy", "port::t", 5, 25, "prod")],
        ),
        ("misc", "q", vec![], vec![]),
    ];
    let mut files = serde_json::Map::new();
    let mut modules = vec![];
    let mut crates = vec![];
    let mut module_brick = serde_json::Map::new();
    let brick_name = |pkg: &'static str| match pkg {
        "eng" => Some("engine"),
        "adp" => Some("adapter"),
        pkg @ ("app" | "port") => Some(pkg),
        _ => None,
    };
    let w = World {
        dir,
        cache: Value::Null,
    };
    for (pkg, m, edges, fns) in mods {
        let path = format!("crates/{pkg}/src/{m}.rs");
        let text = source(&path);
        w.write(&format!("ws/{path}"), &text);
        let id = format!("{pkg}::{m}");
        files.insert(
            path.clone(),
            json!({
                "sha256": sha(&text), "compiled": true, "size": text.len(), "lines": 10, "scope": "prod",
                "module": id, "functions": fns, "edges": edges,
            }),
        );
        modules.push(json!({"id": id, "crate": pkg, "file": path, "line": 1}));
        crates.push(
            json!({"id": pkg, "package": pkg, "version": "0.1.0", "target": pkg,
                           "kind": "lib", "root": format!("crates/{pkg}/src/lib.rs")}),
        );
        if let Some(b) = brick_name(pkg) {
            module_brick.insert(id, json!(b));
        }
    }
    let mut cache = json!({
        "schema_version": 4, "tool_version": "0", "rust_analyzer": "0",
        "workspace_root": w.ws().display().to_string(), "git": null,
        "bricks": {"config_hash": brick_hash(&defs()), "defs": [], "module_brick": module_brick},
        "crates": crates, "modules": modules, "files": files,
    });
    mutate(&mut cache);
    World { cache, ..w }
}

pub struct Out {
    pub code: i32,
    pub json: Value,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(config: &Path, extra: &[&str]) -> Out {
    let o = Command::new(BIN)
        .args(["check", "--config"])
        .arg(config)
        .args(extra)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
    Out {
        code: o.status.code().unwrap(),
        json: serde_json::from_str(&stdout).unwrap_or(Value::Null),
        stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

pub fn check(w: &World, config: &str) -> Out {
    run(&w.save(config), &["--format", "json"])
}

pub fn with_bricks(rest: &str) -> String {
    format!("{BRICKS}\n{rest}")
}

/// Diagnostics whose code starts with `prefix`.
pub fn codes<'a>(o: &'a Out, prefix: &str) -> Vec<&'a Value> {
    o.json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"].as_str().unwrap().starts_with(prefix))
        .collect()
}
