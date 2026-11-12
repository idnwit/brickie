//! The code viewer adapter, through the mounted `/code` router: trees, files, marks and
//! navigation over the test cache plus a second brick `app` using `busy` from `engine`.

use super::*;
use codeview_port::{FileText, Location, Mark, NodeKind, View};

const USER: &str = "use crate::x::busy;\nfn f() { busy(); }\n";

/// The test site with `crates/eng/src/y.rs` (module `eng::y`, brick `app`) calling `busy`,
/// and `x.rs` calling it too.
fn site() -> Site {
    let s = Site::new();
    let dir = s.config().parent().unwrap().to_path_buf();
    std::fs::write(dir.join("ws/crates/eng/src/y.rs"), USER).unwrap();
    let cfg = format!(
        "{CONFIG}\n[[brick]]\nname = \"app\"\ntags = [\"svc\"]\nmembers = [\"eng::y\"]\n\n[deps]\ndefault = \"deny\"\n"
    );
    std::fs::write(s.config(), cfg).unwrap();
    let path = dir.join(".brickie/cache.json");
    let mut cache: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let edge = |from: &str, line: u32, col: u32| {
        json!({"from": from, "to": "eng::x", "kind": "uses", "scope": "prod", "line": line,
               "col": col, "end_col": col + 4, "symbol": "x::busy", "written": "busy"})
    };
    cache["modules"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "eng::y", "crate": "eng", "file": "crates/eng/src/y.rs", "line": 1}));
    let files = cache["files"].as_object_mut().unwrap();
    files.insert(
        "crates/eng/src/y.rs".into(),
        json!({"sha256": "old", "compiled": true, "size": USER.len(), "lines": 2,
               "scope": "prod", "module": "eng::y", "functions": [],
               "edges": [edge("eng::y", 1, 15), edge("eng::y", 2, 10)], "items": []}),
    );
    files.insert(
        "crates/eng/src/deep/z.rs".into(),
        json!({"sha256": "old", "compiled": true, "size": 0, "lines": 0, "scope": "prod",
               "module": "eng::y", "functions": [], "edges": [], "items": []}),
    );
    files["crates/eng/src/x.rs"]["edges"] = json!([edge("eng::x", 3, 1)]);
    std::fs::write(&path, cache.to_string()).unwrap();
    s
}

async fn api<T: serde::de::DeserializeOwned>(s: &Site, ep: &str, body: Value) -> T {
    let (st, b) = s.post(&format!("/code/api/{ep}"), body).await;
    assert_eq!(st, StatusCode::OK, "{ep}: {b}");
    serde_json::from_str(&b).unwrap()
}

#[tokio::test]
async fn trees_group_files_by_brick_and_folder() {
    let s = site();
    let views: Vec<View> = api(&s, "views", Value::Null).await;
    let names: Vec<&str> = views.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["Bricks", "Folders"]);
    let engine = &views[0].roots[0];
    assert_eq!(
        (engine.id.as_str(), engine.label.as_str()),
        ("root/engine", "engine")
    );
    let krate = &engine.children[0];
    assert_eq!(krate.label, "eng");
    assert_eq!(
        (krate.kind, engine.kind),
        (NodeKind::Crate, NodeKind::Brick)
    );
    // Files sit in a folder tree below the crate, not as "src/x.rs" labels.
    let src = &krate.children[0];
    assert_eq!((src.label.as_str(), src.kind), ("src", NodeKind::Folder));
    assert_eq!(src.id, "root/engine#eng/src/");
    let files: Vec<&str> = src.children.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(files, ["link.rs", "x.rs"]);
    assert!(src.children.iter().all(|f| f.kind == NodeKind::File));
    // The complexity error on `busy` is counted on the file and summed up the tree.
    assert_eq!(engine.badge.as_ref().unwrap().errors, 1);
    let app = &views[0].roots[1];
    assert_eq!(app.tags, ["svc"]);
    // Nested folders: crate -> src -> deep -> z.rs, with y.rs beside `deep`.
    let src = &app.children[0].children[0];
    let deep = &src.children[0];
    assert_eq!((deep.label.as_str(), deep.kind), ("deep", NodeKind::Folder));
    assert_eq!(
        deep.children[0].path.as_deref(),
        Some("crates/eng/src/deep/z.rs")
    );
    assert_eq!(src.children[1].path.as_deref(), Some("crates/eng/src/y.rs"));
    let top = &views[1].roots[0];
    assert_eq!((top.id.as_str(), top.label.as_str()), ("crates/", "crates"));
}

#[tokio::test]
async fn files_are_confined_and_stale_is_reported() {
    let s = site();
    let f: FileText = api(&s, "file", json!({"path": "crates/eng/src/x.rs"})).await;
    assert!(!f.stale);
    assert_eq!(f.lang, "rust");
    let y: FileText = api(&s, "file", json!({"path": "crates/eng/src/y.rs"})).await;
    assert!(y.stale, "the cached hash differs");
    for (path, want) in [
        ("crates/eng/src/other.rs", StatusCode::NOT_FOUND),
        ("crates/eng/src/link.rs", StatusCode::NOT_FOUND),
        ("../secret.txt", StatusCode::BAD_REQUEST),
    ] {
        let (st, body) = s.post("/code/api/file", json!({"path": path})).await;
        assert_eq!(st, want, "{path}");
        assert!(!body.contains("top secret"));
    }
}

#[tokio::test]
async fn marks_are_findings_with_links() {
    let s = site();
    let marks: Vec<Mark> = api(&s, "marks", json!({"path": "crates/eng/src/x.rs"})).await;
    let cx = marks
        .iter()
        .find(|m| m.message.starts_with("complexity"))
        .unwrap();
    assert_eq!(cx.span.start.line, 2);
    assert_eq!(cx.level, codeview_port::Level::Error);
    let id = s.diag_id();
    assert_eq!(cx.link.as_deref(), Some(format!("/?item={id}").as_str()));
    // The denied app -> engine use is marked at its sites in y.rs.
    let marks: Vec<Mark> = api(&s, "marks", json!({"path": "crates/eng/src/y.rs"})).await;
    assert!(
        marks.iter().any(|m| m.message.starts_with("deps/")),
        "{marks:?}"
    );
}

#[tokio::test]
async fn definition_and_references_follow_the_edges() {
    let s = site();
    let at =
        |path: &str, line: u32, col: u32| json!({"path": path, "pos": {"line": line, "col": col}});
    let defs: Vec<Location> = api(&s, "definition", at("crates/eng/src/y.rs", 2, 11)).await;
    assert_eq!(defs.len(), 1);
    assert_eq!(
        (defs[0].path.as_str(), defs[0].span.start.line),
        ("crates/eng/src/x.rs", 2)
    );
    assert!(defs[0].label.contains("pub fn busy()"), "{}", defs[0].label);
    // From the declaration itself (`pub fn busy` on line 2 of x.rs): the uses, app first.
    let refs: Vec<Location> = api(&s, "references", at("crates/eng/src/x.rs", 2, 9)).await;
    let got: Vec<(&str, u32, &str)> = refs
        .iter()
        .map(|r| (r.path.as_str(), r.span.start.line, r.label.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("crates/eng/src/y.rs", 1, "root/app"),
            ("crates/eng/src/y.rs", 2, "root/app"),
            ("crates/eng/src/x.rs", 3, "root/engine (same brick)"),
        ]
    );
    let none: Vec<Location> = api(&s, "references", at("crates/eng/src/x.rs", 1, 1)).await;
    assert!(none.is_empty());
}

#[tokio::test]
async fn bands_show_each_function_against_its_limit() {
    let s = site();
    let bands: Vec<codeview_port::Band> =
        api(&s, "bands", json!({"path": "crates/eng/src/x.rs"})).await;
    let busy = bands.iter().find(|b| b.label == "fn busy").unwrap();
    assert_eq!((busy.start_line, busy.depth), (2, 0));
    // cognitive 20 against the limit 5: as hot as it gets.
    assert_eq!(busy.heat, 1.0);
    assert!(
        busy.detail.contains("cognitive 20 / limit 5 (over)"),
        "{}",
        busy.detail
    );
    let quiet = bands.iter().find(|b| b.label == "fn quiet").unwrap();
    assert!(quiet.heat < 0.5, "{}", quiet.heat);
    let none: Vec<codeview_port::Band> =
        api(&s, "bands", json!({"path": "crates/eng/src/nope.rs"})).await;
    assert!(none.is_empty());
}
