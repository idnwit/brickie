//! `Interface` and `EdgeUses` endpoints.

use super::*;

#[tokio::test]
async fn interface_lists_declared_items_and_marks_unused() {
    let s = Site::new();
    let (st, body) = s
        .post("/api/interface", json!({"brick": "root/engine"}))
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: brickie_web_api::InterfaceView = serde_json::from_str(&body).unwrap();
    assert_eq!(v.brick, "root/engine");
    let names: Vec<&str> = v.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["busy", "quiet"]);
    assert_eq!(v.items[0].signature, "pub fn busy()");
    assert_eq!(v.unused(), 2);
    // The same items are `interface/unused` findings.
    let d: Value = serde_json::from_str(&s.get("/api/diagnostics?code=interface").await.1).unwrap();
    assert_eq!(d.as_array().unwrap().len(), 2);

    let (st, body) = s
        .post("/api/interface", json!({"brick": "root/nope"}))
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(body.contains("no brick"));
}

#[tokio::test]
async fn edge_uses_needs_known_bricks() {
    let s = Site::new();
    let (st, body) = s
        .post(
            "/api/edge-uses",
            json!({"from": "root/engine", "to": "root/engine"}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(body, "[]");
    let (st, _) = s
        .post(
            "/api/edge-uses",
            json!({"from": "root/x", "to": "root/engine"}),
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
