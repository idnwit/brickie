//! Links on the levels. View mode: every declared link, the ones no measured edge uses marked
//! planned. Design mode (the editor) serves the same tree and draws every link, judged by the
//! rules (denied, cycle), also towards bricks without code; with no cache it falls back to the
//! design files with a notice.

use super::*;

/// The link `from -> to` of the root level of `levels`.
fn link<'a>(levels: &'a Value, from: &str, to: &str) -> &'a Value {
    (levels[0]["links"].as_array().unwrap().iter())
        .find(|e| e["from"] == from && e["to"] == to)
        .unwrap_or_else(|| panic!("no {from} -> {to} link: {levels}"))
}

#[tokio::test]
async fn design_mode_draws_every_link_towards_a_brick_without_code() {
    let s = Site::new();
    let ops = json!([
        {"op": "add_brick", "file": "brickie.toml",
         "brick": {"name": "cache", "members": ["cache::x"]}},
        {"op": "add_rule", "file": "brickie.toml",
         "rule": {"from": "engine", "to": "cache", "reason": "engine fills the cache"}},
        {"op": "add_link", "file": "brickie.toml",
         "link": {"from": "engine", "to": "cache", "kinds": ["implements"],
                  "scope": "test", "reason": "engine fills the cache"}},
        {"op": "add_link", "file": "brickie.toml",
         "link": {"from": "cache", "to": "engine", "reason": "no rule allows this"}}
    ]);
    let (st, text) = s
        .post("/api/design/edit", json!({"base": 1, "ops": ops}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let edge = link(&r["levels"], "engine", "cache");
    assert_eq!(edge["file"], "brickie.toml");
    assert_eq!(edge["filter"], "`implements` edge in test scope");
    assert_eq!(edge["reason"], "engine fills the cache");
    assert!(edge.get("denied").is_none(), "{edge}");
    let back = link(&r["levels"], "cache", "engine");
    assert!(back["denied"].is_string(), "{back}");
    assert_eq!(back["cycle"], true, "{back}");

    // Edit mode and view mode both serve the draft (the working design).
    let (st, body) = s.get("/api/graph?edit=1").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(link(&v, "engine", "cache"), edge);
    let (st, body) = s.get("/api/graph").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(body.contains("\"cache\""), "{body}");
}

#[tokio::test]
async fn design_mode_without_a_cache_falls_back_to_the_design_files_with_a_notice() {
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
    assert!(html.contains("showing the design only"), "{html}");
    assert!(shown(&html).contains("data-id=\"engine\""), "{html}");
    // View mode has nothing to show without a cache.
    let (st, _) = s.get("/api/graph").await;
    assert_ne!(st, StatusCode::OK);
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

/// The markup of a page without its `<style>` and `<script>` blocks.
fn shown(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(i) = rest.find("<s") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = ["</style>", "</script>"]
            .iter()
            .filter(|_| tail.starts_with("<style") || tail.starts_with("<script"))
            .find_map(|e| tail.find(e).map(|j| j + e.len()));
        match end {
            Some(j) => rest = &tail[j..],
            None => {
                out.push_str("<s");
                rest = &tail[2..];
            }
        }
    }
    out.push_str(rest);
    out
}
