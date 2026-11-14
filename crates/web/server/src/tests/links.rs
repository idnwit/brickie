//! Planned links are served on the levels with their state and filter text, in view mode and
//! in the editor alike (`brickie-check` computes the state).

use super::*;

#[tokio::test]
async fn a_rule_towards_a_brick_without_code_is_served_as_planned() {
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
    let edge = &r["levels"][0]["planned_edges"][0];
    assert_eq!(edge["state"], "planned", "{edge}");
    assert_eq!(edge["file"], "brickie.toml");
    assert_eq!(edge["filter"], "no `implements` edge in test scope");
    assert_eq!(edge["reason"], "engine fills the cache");
    assert!(r["design"].get("planned_edges").is_none());

    // View mode serves the same link, without the editor.
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v[0]["planned_edges"][0], *edge);
}
