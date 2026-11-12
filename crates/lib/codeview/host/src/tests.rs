use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use codeview_port::*;
use http_body_util::BodyExt;
use std::sync::Mutex;
use tower::ServiceExt;

/// A provider with one file, recording every path it is asked for.
#[derive(Default)]
struct Fake {
    asked: Mutex<Vec<String>>,
}

fn at(line: u32) -> Span {
    Span {
        start: Pos { line, col: 1 },
        end: Pos { line, col: 4 },
    }
}

impl Provider for Fake {
    fn views(&self) -> Vec<View> {
        vec![View {
            name: "Folders".into(),
            roots: vec![],
        }]
    }
    fn file(&self, path: &str) -> Option<FileText> {
        self.asked.lock().unwrap().push(path.into());
        (path == "src/a.rs").then(|| FileText {
            path: path.into(),
            text: "fn a() {}".into(),
            lang: "rust".into(),
            stale: false,
        })
    }
    fn marks(&self, path: &str) -> Vec<Mark> {
        vec![Mark {
            span: at(1),
            level: Level::Error,
            message: path.into(),
            link: None,
        }]
    }
    fn bands(&self, _: &str) -> Vec<Band> {
        vec![Band {
            start_line: 1,
            end_line: 3,
            depth: 0,
            heat: 0.25,
            label: "fn a".into(),
            detail: String::new(),
        }]
    }
    fn definition(&self, path: &str, pos: Pos) -> Vec<Location> {
        vec![Location {
            path: path.into(),
            span: at(pos.line),
            label: "def".into(),
        }]
    }
    fn references(&self, _: &str, _: Pos) -> Vec<Location> {
        vec![]
    }
}

async fn call(app: Router, path: &str, body: &str) -> (StatusCode, String) {
    let req = Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn endpoints_answer_from_the_provider() {
    let app = router(Arc::new(Fake::default()));
    let (s, b) = call(app.clone(), "/api/views", "null").await;
    assert_eq!(s, StatusCode::OK);
    assert!(b.contains("Folders"), "{b}");
    let (s, b) = call(app.clone(), "/api/file", r#"{"path":"src/a.rs"}"#).await;
    assert_eq!(s, StatusCode::OK);
    let f: FileText = serde_json::from_str(&b).unwrap();
    assert_eq!(f.text, "fn a() {}");
    let (s, _) = call(app.clone(), "/api/file", r#"{"path":"src/b.rs"}"#).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, b) = call(app.clone(), "/api/marks", r#"{"path":"src/a.rs"}"#).await;
    assert!(b.contains("\"error\""), "{b}");
    let (_, b) = call(app.clone(), "/api/bands", r#"{"path":"src/a.rs"}"#).await;
    let bands: Vec<Band> = serde_json::from_str(&b).unwrap();
    assert_eq!(bands[0].label, "fn a");
    let at = r#"{"path":"src/a.rs","pos":{"line":7,"col":2}}"#;
    let (_, b) = call(app.clone(), "/api/definition", at).await;
    let locs: Vec<Location> = serde_json::from_str(&b).unwrap();
    assert_eq!(locs[0].span.start.line, 7);
    let (s, b) = call(app, "/api/references", at).await;
    assert_eq!((s, b.as_str()), (StatusCode::OK, "[]"));
}

#[tokio::test]
async fn traversal_never_reaches_the_provider() {
    let fake = Arc::new(Fake::default());
    let app = router(fake.clone());
    for bad in ["../secret", "/etc/passwd", "a/../../b", "a\\\\b", ""] {
        let body = format!(r#"{{"path":"{bad}"}}"#);
        for ep in ["/api/file", "/api/marks", "/api/bands"] {
            let (s, _) = call(app.clone(), ep, &body).await;
            assert_eq!(s, StatusCode::BAD_REQUEST, "{ep} {bad}");
        }
    }
    assert!(fake.asked.lock().unwrap().is_empty());
}

#[tokio::test]
async fn page_and_static_files() {
    let app = router(Arc::new(Fake::default()));
    let res = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let page = String::from_utf8_lossy(&bytes);
    if embedded() {
        assert!(page.contains("codeview_app.js"), "{page}");
    } else {
        assert!(page.contains("scripts/build-codeview.sh"), "{page}");
    }
    let res = app
        .oneshot(Request::get("/static/nope.js").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
