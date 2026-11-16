//! The draft's lifecycle from the viewer: publish (blocked while the code lacks the draft's
//! structure, dry run with the gate), rebase onto a newer published head, and discard.

use super::*;

fn rename() -> Value {
    json!({"op": "rename_brick", "file": "brickie.toml", "from": "engine", "to": "core"})
}

/// An edit based on `base`; answers the design after it.
async fn edit(s: &Site, base: u64, op: Value) -> Value {
    let (st, text) = s
        .post("/api/design/edit", json!({"base": base, "ops": [op]}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    serde_json::from_str::<Value>(&text).unwrap()["design"].clone()
}

/// A site whose code matches its cache (the shared fixture has an unmeasured file).
fn measured() -> Site {
    let s = Site::new();
    let other = s.config().with_file_name("ws/crates/eng/src/other.rs");
    std::fs::remove_file(other).unwrap();
    s
}

fn published(s: &Site) -> String {
    std::fs::read_to_string(s.config()).unwrap()
}

#[tokio::test]
async fn a_draft_with_planned_bricks_is_blocked_and_the_dry_run_lists_why() {
    let s = measured();
    let add = json!({"op": "add_brick", "file": "brickie.toml",
                     "brick": {"name": "cache", "members": ["cache::x"]}});
    let d = edit(&s, 1, add).await;
    let base = d["v"].as_u64().unwrap();

    let (st, text) = s
        .post(
            "/api/design/publish",
            json!({"base": base, "dry_run": true}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(r["changed"], false);
    let blocking = &r["design"]["gate"]["blocking"];
    assert_eq!(blocking[0]["code"], "design/planned", "{blocking}");
    assert_eq!(blocking[0]["brick"], "root/cache");

    let (st, text) = s.post("/api/design/publish", json!({"base": base})).await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert!(
        text.contains("design/blocked") && text.contains("cache"),
        "{text}"
    );
    assert!(!published(&s).contains("cache"), "nothing published");
}

#[tokio::test]
async fn a_ready_draft_is_published_and_dropped() {
    let s = measured();
    let d = edit(&s, 1, rename()).await;
    assert_eq!(d["gate"]["blocking"], json!([]), "{d}");
    let base = d["v"].as_u64().unwrap();

    let (st, text) = s
        .post("/api/design/publish", json!({"base": base, "note": "ship"}))
        .await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let r: Value = serde_json::from_str(&text).unwrap();
    let d = &r["design"];
    assert_eq!(r["changed"], true);
    assert_eq!(d["v"], d["published"], "the published head is the head");
    assert!(
        d.get("forked_from").is_none() && d.get("gate").is_none(),
        "{d}"
    );
    assert!(published(&s).contains("\"core\""));

    // View mode now shows the published rename.
    let (_, body) = s.get("/api/graph").await;
    assert!(body.contains("\"core\""), "{body}");
}

#[tokio::test]
async fn a_draft_behind_the_published_head_is_rebased() {
    let s = measured();
    let d = edit(&s, 1, rename()).await;
    let base = d["v"].as_u64().unwrap();
    // A hand edit of the published file: a newer published version.
    std::fs::write(s.config(), CONFIG.replace("cognitive = 5", "cognitive = 6")).unwrap();

    let (_, text) = s.get("/api/design").await;
    let d: Value = serde_json::from_str(&text).unwrap();
    let head = d["published"].as_u64().unwrap();
    assert!(head > 1 && d["forked_from"] == 1, "behind: {d}");
    assert!(d.get("gate").is_none(), "no gate while behind: {d}");
    let (st, _) = s.post("/api/design/publish", json!({"base": base})).await;
    assert_eq!(st, StatusCode::CONFLICT, "a draft behind is not published");

    let (st, text) = s.post("/api/design/rebase", json!({"base": base})).await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let d = &serde_json::from_str::<Value>(&text).unwrap()["design"];
    assert_eq!(d["forked_from"].as_u64(), Some(head), "{d}");
    assert_eq!(d["gate"]["blocking"], json!([]), "{d}");
    let draft = s
        .config()
        .with_file_name(".brickie/draft/files/brickie.toml");
    let text = std::fs::read_to_string(draft).unwrap();
    assert!(
        text.contains("\"core\"") && text.contains("cognitive = 6"),
        "{text}"
    );
}

#[tokio::test]
async fn discard_drops_the_draft_and_keeps_the_published_design() {
    let s = measured();
    let d = edit(&s, 1, rename()).await;
    let base = d["v"].as_u64().unwrap();
    let (st, text) = s.post("/api/design/discard", json!({"base": base})).await;
    assert_eq!(st, StatusCode::OK, "{text}");
    let d = &serde_json::from_str::<Value>(&text).unwrap()["design"];
    assert!(d.get("forked_from").is_none(), "{d}");
    assert_eq!(published(&s), CONFIG);
    assert!(!s.config().with_file_name(".brickie/draft").exists());

    // A stale base is refused.
    let (st, text) = s.post("/api/design/discard", json!({"base": base})).await;
    assert_eq!(st, StatusCode::CONFLICT, "{text}");
}
