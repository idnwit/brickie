//! The design editor's endpoints: the design at its head, writes as human versions (edit,
//! move, undo) based on a version, `design/stale` as 409 with the head, link checks, layout
//! hints, and the write guard first.

use super::*;
use brickie_change::{By, Design, Op};

fn add(name: &str) -> Value {
    json!({"op": "add_brick", "file": "brickie.toml", "brick": {"name": name, "members": [format!("{name}::x")]}})
}

/// `(status, x-brickie-head, body)` of a POST from the page.
async fn post_head(s: &Site, path: &str, body: Value) -> (StatusCode, Option<String>, String) {
    let req = Request::post(path)
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3000")
        .header("origin", "http://127.0.0.1:3000")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = router(AppState::new(s.config()))
        .oneshot(req)
        .await
        .unwrap();
    let st = resp.status();
    let head = resp
        .headers()
        .get("x-brickie-head")
        .map(|v| v.to_str().unwrap().to_string());
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (st, head, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn an_edit_is_a_version_and_a_stale_base_is_refused() {
    let s = Site::new();
    let (st, text) = s.get("/api/design").await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let d: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        (d["v"].as_u64(), d["files"]["root"].as_str()),
        (Some(1), Some("brickie.toml"))
    );

    let (st, text) = s
        .post(
            "/api/design/edit",
            json!({"base": 1, "ops": [add("cache")]}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        (r["changed"].as_bool(), r["design"]["v"].as_u64()),
        (Some(true), Some(2))
    );
    assert_eq!(r["design"]["by"], "human");
    let root = &r["levels"][0]["graph"]["bricks"];
    let cache = root
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "cache")
        .unwrap();
    assert_eq!(cache["planned"], true, "no code yet: planned");
    assert!(
        std::fs::read_to_string(s.config())
            .unwrap()
            .contains("cache")
    );

    // Based on v1 while the head is v2: nothing written, 409 with the stale text and the head.
    let (st, head, text) = post_head(
        &s,
        "/api/design/edit",
        json!({"base": 1, "ops": [add("other")]}),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(head.as_deref(), Some("2"));
    assert!(
        text.contains("design/stale") && text.contains("v2"),
        "{text}"
    );
    assert!(
        !std::fs::read_to_string(s.config())
            .unwrap()
            .contains("other")
    );

    // An op that does not apply is a 409 without a head.
    let gone = json!({"op": "remove_brick", "file": "brickie.toml", "name": "nope"});
    let (st, head, _) = post_head(&s, "/api/design/edit", json!({"base": 2, "ops": [gone]})).await;
    assert_eq!((st, head), (StatusCode::CONFLICT, None));
}

#[tokio::test]
async fn undo_reverts_a_version_as_a_new_one() {
    let s = Site::new();
    s.get("/api/design").await;
    s.post(
        "/api/design/edit",
        json!({"base": 1, "ops": [add("cache")]}),
    )
    .await;
    let (st, text) = s.post("/api/design/undo", json!({"base": 2})).await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(r["design"]["v"].as_u64(), Some(3));
    let last = r["design"]["history"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(
        (last["note"].as_str(), last["by"].as_str()),
        (Some("undo v2"), Some("human"))
    );
    assert!(
        !std::fs::read_to_string(s.config())
            .unwrap()
            .contains("cache")
    );
}

#[tokio::test]
async fn an_agent_version_shows_up_and_the_page_edits_on_top() {
    let s = Site::new();
    let d = Design::at(&s.config()).unwrap();
    let head = d.head().unwrap().v;
    let ops = vec![serde_json::from_value::<Op>(add("tel")).unwrap()];
    d.apply(head, By::Agent, None, ops).unwrap();
    let (_, text) = s.get("/api/design").await;
    let st: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        (st["v"].as_u64(), st["by"].as_str()),
        (Some(2), Some("agent"))
    );
    let (code, _) = s
        .post(
            "/api/design/edit",
            json!({"base": 2, "ops": [add("cache")]}),
        )
        .await;
    assert_eq!(code, StatusCode::OK);
}

#[tokio::test]
async fn links_and_link_checks_read_the_live_design() {
    let s = Site::new();
    s.post(
        "/api/design/edit",
        json!({"base": 1, "ops": [add("cache")]}),
    )
    .await;
    let (st, text) = s
        .post("/api/design/links", json!({"path": ["cache"]}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let opts: Value = serde_json::from_str(&text).unwrap();
    assert!(
        opts.as_array().unwrap().iter().any(|o| o["to"] == "engine"),
        "{text}"
    );
    let chk = json!({"from": "cache", "to": "engine", "scope": "prod", "action": "allow"});
    let (st, text) = s.post("/api/design/link-check", chk).await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let c: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(c["redundant"], true, "the default allows it: {text}");
}

#[tokio::test]
async fn layout_hints_live_beside_the_history_and_auto_layout_clears_a_level() {
    let s = Site::new();
    let hint = json!({"brick": "root/engine", "hint": {"x": 10.0, "y": 20.0}});
    assert_eq!(s.post("/api/design/layout", hint).await.0, StatusCode::OK);
    let file = s.config().parent().unwrap().join(".brickie/layout.json");
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("root/engine")
    );
    let (_, text) = s.get("/api/design").await;
    let d: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(d["layout"]["root/engine"]["x"], 10.0);
    // No version: a position is not a design change.
    assert_eq!(d["v"].as_u64(), Some(1));
    let (st, _) = s
        .post("/api/design/relayout", json!({"level": "root"}))
        .await;
    assert_eq!(st, StatusCode::OK);
    let (_, text) = s.get("/api/design").await;
    assert!(!text.contains("root/engine"), "{text}");
}

#[tokio::test]
async fn writes_need_the_same_origin() {
    let s = Site::new();
    let evil = [("origin", "http://evil.test")];
    let (st, _) = s
        .post_with("/review", json!({"id": "deadbeef", "reason": null}), &evil)
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = s
        .post_with(
            "/api/design/edit",
            json!({"base": 1, "ops": [add("x")]}),
            &evil,
        )
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Not bound to loopback: every write is refused.
    let wide = AppState::new(s.config()).bound("0.0.0.0", 3000);
    let req = Request::post("/review")
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3000")
        .body(Body::from(json!({"id": "x", "reason": null}).to_string()))
        .unwrap();
    let resp = router(wide).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_stale_write_that_clashes_is_refused_without_the_stale_text() {
    let s = Site::new();
    let (st, text) = s
        .post(
            "/api/design/edit",
            json!({"base": 1, "ops": [add("cache")]}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    // Same brick on the old base: a conflict naming the writer, no retry hint.
    let (st, head, text) = post_head(
        &s,
        "/api/design/edit",
        json!({"base": 1, "ops": [add("cache")]}),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(head, None);
    assert!(
        text.contains("human v2 changed brick cache") && !text.contains("design/stale"),
        "{text}"
    );
}

#[tokio::test]
async fn a_stale_rule_on_a_removed_brick_is_a_conflict_not_invalid() {
    let s = Site::new();
    for (base, op) in [
        (1, add("store")),
        (
            2,
            json!({"op": "remove_brick", "file": "brickie.toml", "name": "store"}),
        ),
    ] {
        let (st, text) = s
            .post("/api/design/edit", json!({"base": base, "ops": [op]}))
            .await;
        assert_eq!(st, StatusCode::OK, "{text}");
    }
    let rule = json!({"op": "add_rule", "file": "brickie.toml",
        "rule": {"from": "web", "to": "store", "action": "allow"}});
    let (st, _, text) = post_head(&s, "/api/design/edit", json!({"base": 2, "ops": [rule]})).await;
    assert_eq!(st, StatusCode::CONFLICT, "{text}");
    assert!(
        text.contains("changed brick store") && !text.contains("design/"),
        "{text}"
    );
}

#[tokio::test]
async fn a_stale_undo_retries_only_when_nothing_since_touched_the_version() {
    let s = Site::new();
    for (base, name) in [(1, "cache"), (2, "other")] {
        let (st, text) = s
            .post(
                "/api/design/edit",
                json!({"base": base, "ops": [add(name)]}),
            )
            .await;
        assert_eq!(st, StatusCode::OK, "{text}");
    }
    // v3 touched another brick: undoing v2 from base 2 may be re-sent on the head.
    let (st, head, text) = post_head(&s, "/api/design/undo", json!({"base": 2, "v": 2})).await;
    assert_eq!(
        (st, head.as_deref()),
        (StatusCode::CONFLICT, Some("3")),
        "{text}"
    );
    assert!(text.contains("design/stale"), "{text}");
    // v4 removes the brick v2 added: a conflict naming the writer, no retry hint.
    let gone = json!({"op": "remove_brick", "file": "brickie.toml", "name": "cache"});
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 3, "ops": [gone]}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let (st, head, text) = post_head(&s, "/api/design/undo", json!({"base": 2, "v": 2})).await;
    assert_eq!((st, head), (StatusCode::CONFLICT, None));
    assert!(
        text.contains("human v4 changed brick cache") && !text.contains("design/stale"),
        "{text}"
    );
}
