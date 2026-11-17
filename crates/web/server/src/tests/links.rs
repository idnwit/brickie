//! Rule links on the levels. View mode: the allow rules no measured edge backs (unused links).
//! Design mode (the editor): every rule between named bricks from the design files alone, allow
//! or deny, also towards bricks without code, and with no measured cache at all.

use super::*;

/// The rule link `from -> to` of the root level of `levels`.
fn link<'a>(levels: &'a Value, from: &str, to: &str) -> &'a Value {
    (levels[0]["unused_links"].as_array().unwrap().iter())
        .find(|e| e["from"] == from && e["to"] == to)
        .unwrap_or_else(|| panic!("no {from} -> {to} link: {levels}"))
}

#[tokio::test]
async fn design_mode_draws_every_rule_towards_a_brick_without_code() {
    let s = Site::new();
    let ops = json!([
        {"op": "add_brick", "file": "brickie.toml",
         "brick": {"name": "cache", "members": ["cache::x"]}},
        {"op": "add_rule", "file": "brickie.toml",
         "rule": {"from": "engine", "to": "cache", "action": "allow", "kinds": ["implements"],
                  "scope": "test", "reason": "engine fills the cache"}},
        {"op": "add_rule", "file": "brickie.toml",
         "rule": {"from": "cache", "to": "engine", "action": "deny", "reason": "one way"}}
    ]);
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 1, "ops": ops}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let edge = link(&r["levels"], "engine", "cache");
    assert_eq!(edge["file"], "brickie.toml");
    assert_eq!(edge["action"], "allow");
    assert_eq!(edge["filter"], "`implements` edge in test scope");
    assert_eq!(edge["reason"], "engine fills the cache");
    assert_eq!(link(&r["levels"], "cache", "engine")["action"], "deny");
    assert!(
        (r["levels"].as_array().unwrap().iter())
            .all(|lv| lv["graph"]["edges"].as_array().is_none_or(|a| a.is_empty())),
        "no measured edge in Design mode: {}",
        r["levels"]
    );

    // Edit mode serves it from the draft; view mode shows the published design, without it.
    let (st, body) = s.get("/api/graph?edit=1").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(link(&v, "engine", "cache"), edge);
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(!body.contains("\"cache\""), "{body}");
}

#[tokio::test]
async fn design_mode_never_reads_the_measured_cache() {
    let s = Site::new();
    std::fs::remove_file(s.config().with_file_name(".brickie/cache.json")).unwrap();
    let (st, text) = s.get("/api/design").await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let d: Value = serde_json::from_str(&text).unwrap();
    assert!(d.get("error").is_none(), "{d}");
    let (st, body) = s.get("/api/graph?edit=1").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(body.contains("\"engine\""), "{body}");
    let (st, html) = s.get("/?edit=1").await;
    assert_eq!(st, StatusCode::OK, "{html}");
    let add = json!({"op": "add_brick", "file": "brickie.toml", "brick": {"name": "cache"}});
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 1, "ops": [add]}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let (st, text) = s.post("/api/design/levels", json!({})).await;
    assert_eq!(st, StatusCode::OK, "{text}");
    // Only Publish compares code with the design: without a cache it is blocked.
    let (st, text) = s
        .post("/api/design/publish", json!({"base": 2, "dry_run": true}))
        .await;
    assert_eq!(st, StatusCode::CONFLICT, "{text}");
}
