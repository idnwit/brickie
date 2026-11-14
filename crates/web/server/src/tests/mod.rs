//! Route tests: axum router driven through tower `oneshot` on a temp workspace with a
//! hand-built cache (no rust-analyzer).

mod api;
mod code;
mod editor;
mod events;
mod index;
mod interface;
mod levels;
mod levels_snapshot;
mod measure;
mod review;

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

pub(crate) const CONFIG: &str = r#"
[[brick]]
name = "engine"
members = ["eng"]
tags = ["core"]

[[complexity]]
bricks = "*"
cognitive = 5
level = "error"
"#;

const SOURCE: &str = "// x.rs <b>&</b>\npub fn busy() {}\npub fn quiet() {}\n";

pub(crate) struct Site {
    dir: tempfile::TempDir,
}

impl Site {
    pub(crate) fn new() -> Site {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(ws.join("crates/eng/src")).unwrap();
        std::fs::write(ws.join("crates/eng/src/x.rs"), SOURCE).unwrap();
        std::fs::write(dir.path().join("secret.txt"), "top secret\n").unwrap();
        std::fs::write(ws.join("crates/eng/src/other.rs"), "// not in the cache\n").unwrap();
        // A cached file that is a symlink out of the workspace.
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            dir.path().join("secret.txt"),
            ws.join("crates/eng/src/link.rs"),
        )
        .unwrap();
        std::fs::write(dir.path().join("brickie.toml"), CONFIG).unwrap();
        let file = |text: &str, functions: Value, items: Value| {
            let sha = brickie_facts::sha256_hex(text.as_bytes());
            json!({"sha256": sha, "compiled": true, "size": text.len(), "lines": 3,
                   "scope": "prod", "module": "eng::x", "functions": functions, "edges": [],
                   "items": items})
        };
        let item = |name: &str, line: u32| {
            json!({"symbol": format!("x::{name}"), "module": "eng::x", "kind": "fn",
                   "name": name, "signature": format!("pub fn {name}()"), "doc": "",
                   "line": line, "vis": "pub", "visible_in": "eng", "exported": true})
        };
        let func = |name: &str, line: u32, cog: u32| {
            json!({"name": name, "qname": name, "module": "eng::x", "line": line, "end_line": line,
                   "lines": 1, "cognitive": cog, "cyclomatic": 1, "nesting": 0, "paths": 1,
                   "params": 0, "scope": "prod"})
        };
        let cache = json!({
            "schema_version": brickie_facts::SCHEMA_VERSION, "tool_version": "0", "rust_analyzer": "0",
            "workspace_root": ws.display().to_string(), "git": null,
            "crates": [{"id": "eng", "package": "eng", "version": "0.1.0", "target": "eng",
                        "kind": "lib", "root": "crates/eng/src/lib.rs", "dir": "crates/eng"}],
            "modules": [{"id": "eng::x", "crate": "eng", "file": "crates/eng/src/x.rs", "line": 1}],
            "files": {
                "crates/eng/src/x.rs": file(
                    SOURCE,
                    json!([func("busy", 2, 20), func("quiet", 3, 1)]),
                    json!([item("busy", 2), item("quiet", 3)]),
                ),
                "crates/eng/src/link.rs": file("top secret\n", json!([]), json!([])),
            },
        });
        std::fs::create_dir_all(dir.path().join(".brickie")).unwrap();
        std::fs::write(dir.path().join(".brickie/cache.json"), cache.to_string()).unwrap();
        Site { dir }
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.dir.path().join("brickie.toml")
    }

    pub(crate) async fn call(&self, req: Request<Body>) -> (StatusCode, String) {
        let resp = router(AppState::new(self.config()))
            .oneshot(req)
            .await
            .unwrap();
        let status = resp.status();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    pub(crate) async fn get(&self, uri: &str) -> (StatusCode, String) {
        self.call(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    pub(crate) async fn review(&self, body: Value) -> (StatusCode, String) {
        self.post("/review", body).await
    }

    /// POST from the page the server served (passes the write guard).
    pub(crate) async fn post(&self, path: &str, body: Value) -> (StatusCode, String) {
        self.post_with(path, body, &[("origin", "http://127.0.0.1:3000")])
            .await
    }

    /// POST with Host `127.0.0.1:3000` and the extra `headers`.
    pub(crate) async fn post_with(
        &self,
        path: &str,
        body: Value,
        headers: &[(&str, &str)],
    ) -> (StatusCode, String) {
        let mut req = Request::post(path)
            .header("content-type", "application/json")
            .header("host", "127.0.0.1:3000");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        self.call(req.body(Body::from(body.to_string())).unwrap())
            .await
    }

    /// The id of the single diagnostic, as `brickie check` computes it.
    pub(crate) fn diag_id(&self) -> String {
        let l = check::load(&self.config()).unwrap();
        let d: Vec<_> = l
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("complexity"))
            .collect();
        assert_eq!(d.len(), 1);
        d[0].id.clone()
    }
}
