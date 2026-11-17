//! The page: markers, layers and presets as params.

use super::*;

#[tokio::test]
async fn index_renders_diagnostic_with_hydration_markers() {
    let s = Site::new();
    let (st, html) = s.get("/").await;
    assert_eq!(st, StatusCode::OK);
    // The default Architecture preset lists no findings; Violations and All do.
    assert!(!html.contains("class=\"fi"), "{html}");
    let all = s.get("/?p=all").await.1;
    assert!(all.contains("`busy` has cognitive 20 (limit 5)"), "{all}");
    // A selected finding shows its card with the code and a source link.
    let id = s.diag_id();
    let html = s.get(&format!("/?p=all&item={id}")).await.1;
    assert!(html.contains("complexity/cognitive"), "{html}");
    // Hydration keys and the embedded data the wasm bundle starts from.
    assert!(html.contains("data-hk="));
    assert!(html.contains("id=\"brickie-data\""));
    assert!(html.contains("id=\"app\""));
    // Spans are links to the code viewer; the graph is SVG.
    assert!(html.contains("/code/?path=crates/eng/src/x.rs&amp;line=2"));
    assert!(html.contains("<svg"));
    assert!(
        html.contains("id=\"stage\"") && html.contains("class=\"node"),
        "{html}"
    );
    assert!(html.contains("findings (E) of any layer"));
    assert!(
        html.contains("class=\"sbadge") && html.contains("class=\"heat h"),
        "{html}"
    );
    // The wasm script is only emitted when a bundle is embedded.
    match crate::routes::bundle() {
        Some((js, wasm)) => assert!(html.contains(&js) && html.contains(&wasm), "{html}"),
        None => assert!(!html.contains("/static/ui/"), "{html}"),
    }
}

#[tokio::test]
async fn filters_change_the_output() {
    let s = Site::new();
    let has_card = |html: &str| html.contains("class=\"fi");
    assert!(has_card(&s.get("/?p=all&code=complexity").await.1));
    assert!(!has_card(&s.get("/?p=all&code=deps").await.1));
    assert!(has_card(&s.get("/?p=all&brick=engine").await.1));
    // The page ignores unknown bricks (as the original viewer ignored unknown ids).
    assert!(has_card(&s.get("/?p=all&brick=nope").await.1));
    // Layers: the complexity layer off hides the finding; Violations shows errors only.
    assert!(!has_card(
        &s.get("/?p=all&lay=-cx,-stale,-deps,-iface").await.1
    ));
    assert!(has_card(&s.get("/?p=viol").await.1));
    assert!(!has_card(&s.get("/?p=viol&vs=deps").await.1));
    // Function sort: by name ascending puts `busy` before `quiet`; by cognitive too, reversed
    // with fn_dir=asc.
    let pos = |html: &str, n: &str| html.find(&format!(">{n}</a>")).unwrap();
    let desc = s.get("/?fn_sort=cognitive").await.1;
    assert!(pos(&desc, "busy") < pos(&desc, "quiet"));
    let asc = s.get("/?fn_sort=cognitive&fn_dir=asc").await.1;
    assert!(pos(&asc, "quiet") < pos(&asc, "busy"));
    // The active filter is reflected in the links.
    assert!(
        s.get("/?code=complexity")
            .await
            .1
            .contains("class=\"chip on\"")
    );
}

#[test]
fn bundle_detection_follows_the_build() {
    assert_eq!(crate::routes::wasm_embedded(), cfg!(ui_wasm));
}

#[test]
fn failure_is_plain_text_with_its_status() {
    use axum::response::IntoResponse;
    let r = crate::Failure(StatusCode::NOT_FOUND, "nope".into()).into_response();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(r.headers()["content-type"], "text/plain; charset=utf-8");
}
