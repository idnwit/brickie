//! The design editor's endpoints: open, record, preview (planned bricks), links, layout,
//! review, publish plan, discard, and the write guard.

use super::*;
use brickie_change::Change;

async fn json(s: &Site, path: &str, body: Value) -> Value {
    let (st, text) = s.post(path, body).await;
    assert_eq!(st, StatusCode::OK, "{path}: {text}");
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn change_round_trip_through_the_endpoints() {
    let s = Site::new();
    assert_eq!(s.get("/api/change").await.1, "null");
    let v = json(&s, "/api/change/open", json!({"intent": "add a cache"})).await;
    assert_eq!(v["status"]["intent"], "add a cache");
    assert_eq!(v["files"]["root"], "brickie.toml");

    let op = json!({"op": {"op": "add_brick", "file": "brickie.toml",
        "brick": {"name": "cache", "kind": "crate"}}, "note": "from the palette"});
    let rec = json(&s, "/api/change/op", op).await["done"].clone();
    assert_eq!(
        (rec["seq"].clone(), rec["state"].clone()),
        (json!(1), json!("accepted"))
    );
    // The config file is untouched until publish.
    assert!(
        !std::fs::read_to_string(s.config())
            .unwrap()
            .contains("cache")
    );

    // The preview shows the new brick as planned; the current design does not have it.
    let lv: Value = serde_json::from_str(&s.get("/api/graph?change=1").await.1).unwrap();
    let cache = lv[0]["graph"]["bricks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "cache")
        .cloned()
        .expect("planned brick in the preview");
    assert_eq!(cache["planned"], json!(true));
    let cur = s.get("/api/graph").await.1;
    assert!(!cur.contains("\"cache\""));
    let page = s.get("/?change=1").await.1;
    assert!(page.contains("cache"));

    let links = json(&s, "/api/change/links", json!({"path": ["cache"]})).await;
    let first = &links.as_array().unwrap()[0];
    // No `[deps]`: the default allows.
    assert_eq!(
        (first["to"].clone(), first["action"].clone()),
        (json!("engine"), json!("allow"))
    );
    let rule = json!({"op": {"op": "add_rule", "file": "brickie.toml", "rule":
        {"from": "cache", "to": "engine", "action": "allow", "reason": "reads facts"}}});
    json(&s, "/api/change/op", rule).await;

    let hint = json!({"brick": "root/cache", "hint": {"x": 10.0, "y": 20.0}});
    json(&s, "/api/change/layout", hint).await;
    let state: Value = serde_json::from_str(&s.get("/api/change").await.1).unwrap();
    assert_eq!(state["layout"]["root/cache"]["x"], json!(10.0));
    assert_eq!(state["status"]["accepted"], json!(2));

    let rejected = json!({"seq": 2, "verdict": "reject", "note": "not yet"});
    json(&s, "/api/change/review", rejected).await;
    let plan: Value = serde_json::from_str(&s.get("/api/change/plan").await.1).unwrap();
    assert!(plan["files"].as_array().is_some());

    // An op that does not apply is refused and not recorded.
    let bad = json!({"op": {"op": "remove_brick", "file": "brickie.toml", "name": "nope"}});
    assert_eq!(s.post("/api/change/op", bad).await.0, StatusCode::CONFLICT);

    json(&s, "/api/change/discard", json!(null)).await;
    assert_eq!(s.get("/api/change").await.1, "null");
}

#[tokio::test]
async fn writes_need_the_same_origin() {
    let s = Site::new();
    let open = json!({"intent": "x"});
    let evil = [("origin", "http://evil.test")];
    let (st, _) = s.post_with("/api/change/open", open.clone(), &evil).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = s
        .post_with("/review", json!({"id": "deadbeef", "reason": null}), &evil)
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Not bound to loopback: every write is refused.
    let wide = AppState::new(s.config()).bound("0.0.0.0", 3000);
    let req = Request::post("/api/change/open")
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3000")
        .body(Body::from(open.to_string()))
        .unwrap();
    let resp = router(wide).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(!Change::is_open(&s.config()));
}
