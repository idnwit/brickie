//! JSON endpoints.

use super::*;

#[tokio::test]
async fn json_endpoints_use_the_same_params() {
    let s = Site::new();
    let (st, body) = s.get("/api/diagnostics?level=error").await;
    assert_eq!(st, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["code"], "complexity/cognitive");
    assert_eq!(v[0]["snippet"]["hit"], "busy");
    let v: Value =
        serde_json::from_str(&s.get("/api/diagnostics?level=warn&code=complexity").await.1)
            .unwrap();
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
