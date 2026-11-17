//! JSON endpoints.

use super::*;

#[tokio::test]
async fn json_endpoints_use_the_same_params() {
    let s = Site::new();
    let (st, body) = s.get("/api/diagnostics").await;
    assert_eq!(st, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    // Every finding is listed: the over-limit function, two unused items and the file added
    // after measure (a stale cache).
    assert_eq!(v.as_array().unwrap().len(), 4);
    assert_eq!(v[3]["code"], "stale/file");
    assert_eq!(v[0]["code"], "complexity/cognitive");
    assert_eq!(v[0]["snippet"]["hit"], "busy");
    let v: Value = serde_json::from_str(&s.get("/api/diagnostics?code=deps").await.1).unwrap();
    assert!(v.as_array().unwrap().is_empty());
    let v: Value = serde_json::from_str(&s.get("/api/diagnostics?brick=nope").await.1).unwrap();
    assert!(v.as_array().unwrap().is_empty());
    let v: Value =
        serde_json::from_str(&s.get("/api/functions?fn_sort=cognitive&fn_limit=1").await.1)
            .unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["qname"], "busy");
    assert_eq!(v[0]["brick"], "root/engine");
    let v: Value =
        serde_json::from_str(&s.get("/api/functions?brick=root/engine").await.1).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
    let v: Value = serde_json::from_str(&s.get("/api/graph").await.1).unwrap();
    assert_eq!(v[0]["path"], "root");
    let g = &v[0]["graph"];
    assert_eq!(g["bricks"][0]["name"], "engine");
    assert!(g["edges"].is_array() && g["rows"].is_array());
}

#[tokio::test]
async fn design_mode_serves_the_same_measured_data_as_view_mode() {
    let s = Site::new();
    let view: Value = serde_json::from_str(&s.get("/api/graph").await.1).unwrap();
    let edit: Value = serde_json::from_str(&s.get("/api/graph?edit=1").await.1).unwrap();
    assert_eq!(view, edit, "one tree: Design mode only turns editing on");
    // The over-limit function counts as a finding on the engine brick in Design mode too.
    let bricks = edit[0]["graph"]["bricks"].as_array().unwrap();
    assert!(
        bricks[0]["worst_ratio"].as_f64().unwrap() > 1.0,
        "measured heat: {bricks:?}"
    );
    let (_, html) = s.get("/?edit=1").await;
    assert!(
        html.contains("complexity/cognitive"),
        "findings are in the page data: {html}"
    );
    assert!(
        !html.contains("showing the design only"),
        "no fallback with a cache"
    );
}
