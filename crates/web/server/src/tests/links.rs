//! Links on the levels. View mode: every declared link, the ones no measured edge uses marked
//! planned. Design mode (the editor): every link from the design files alone, judged by the
//! rules (denied, cycle), also towards bricks without code, and with no measured cache at all.

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
    // The page shows the design only: no findings, filters, scores or heat. Styles and the
    // hydration state are not shown text, so only the markup between them is read.
    assert!(html.contains("Bricks (as declared)"), "{html}");
    let html = shown(&html);
    assert!(
        html.contains("data-id=\"engine\""),
        "the canvas is rendered: {html}"
    );
    assert!(html.contains("Data layers"), "{html}");
    for code in [
        "id=\"q\"",
        "Bricks by tag",
        "filter text",
        "class=\"scope\"",
        "finding",
        "score",
        "complexity excess",
        "stale",
        "Findings",
        "no complexity limit",
    ] {
        assert!(!html.contains(code), "{code} in Design mode: {html}");
    }
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
