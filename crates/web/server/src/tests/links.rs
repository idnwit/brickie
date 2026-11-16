//! Unused links (allow rules no measured edge backs) are served on the levels with their filter
//! text, in view mode and in the editor alike, also towards bricks without code (a draft:
//! the editor only).

use super::*;

#[tokio::test]
async fn a_rule_towards_a_brick_without_code_is_served_as_unused() {
    let s = Site::new();
    let ops = json!([
        {"op": "add_brick", "file": "brickie.toml",
         "brick": {"name": "cache", "members": ["cache::x"]}},
        {"op": "add_rule", "file": "brickie.toml",
         "rule": {"from": "engine", "to": "cache", "action": "allow", "kinds": ["implements"],
                  "scope": "test", "reason": "engine fills the cache"}}
    ]);
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 1, "ops": ops}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let edge = &r["levels"][0]["unused_links"][0];
    assert!(edge.get("state").is_none(), "{edge}");
    assert_eq!(edge["file"], "brickie.toml");
    assert_eq!(edge["filter"], "no `implements` edge in test scope");
    assert_eq!(edge["reason"], "engine fills the cache");
    assert!(r["design"].get("unused_links").is_none());

    // Edit mode serves it from the draft; view mode shows the published design, without it.
    let (st, body) = s.get("/api/graph?edit=1").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v[0]["unused_links"][0], *edge);
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(
        v[0]["unused_links"].as_array().is_none_or(|a| a.is_empty()),
        "{v}"
    );
}
