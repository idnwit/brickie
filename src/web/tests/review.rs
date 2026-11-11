//! Review round trip, assets and error pages.

use super::*;

#[tokio::test]
async fn review_round_trips_into_the_toml() {
    let s = Site::new();
    let id = s.diag_id();
    let path = s.dir.path().join("brickie.review.toml");
    // Existing content is kept.
    std::fs::write(
        &path,
        "# reviewed by hand\n[[accept]]\nid = \"deadbeef\"\nreason = \"older\"\n",
    )
    .unwrap();

    let (st, body) = s
        .review(json!({"id": id, "reason": "  fine for now "}))
        .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (v["accepted"].clone(), v["reason"].clone()),
        (json!(true), json!("fine for now"))
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# reviewed by hand"));
    assert!(text.contains("deadbeef"));
    assert!(text.contains(&format!("id = \"{id}\"")) && text.contains("fine for now"));
    // `check` reads the same file, and the page now shows it as accepted.
    assert!(
        check::load(&s.config())
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.id == id && d.accepted)
    );
    assert!(
        s.get("/?p=all&status=accepted")
            .await
            .1
            .contains("class=\"fi")
    );
    // Accepting twice keeps one entry.
    s.review(json!({"id": id, "reason": "again"})).await;
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().matches(&id).count(),
        1
    );

    let (st, _) = s.review(json!({"id": id, "reason": null})).await;
    assert_eq!(st, StatusCode::OK);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        !text.contains(&id) && text.contains("deadbeef") && text.contains("# reviewed by hand")
    );
    assert!(
        !check::load(&s.config())
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.accepted && d.id == id)
    );

    // Validation.
    assert_eq!(
        s.review(json!({"id": id, "reason": "  "})).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        s.review(json!({"id": "zz", "reason": "x"})).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        s.review(json!({"id": "00000000", "reason": "x"})).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn review_file_is_created_when_missing() {
    let s = Site::new();
    let id = s.diag_id();
    s.review(json!({"id": id, "reason": "why"})).await;
    let text = std::fs::read_to_string(s.dir.path().join("brickie.review.toml")).unwrap();
    assert!(text.contains("[[accept]]") && text.contains(&id));
}

#[tokio::test]
async fn bundle_routes_match_the_build() {
    let s = Site::new();
    let (st, _) = s.get("/static/ui/brickie_ui_bg.wasm").await;
    assert_eq!(st == StatusCode::OK, cfg!(ui_wasm));
    assert_eq!(s.get("/nope").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_cache_is_a_clear_error() {
    let s = Site::new();
    std::fs::remove_file(s.dir.path().join(".brickie/cache.json")).unwrap();
    let (st, body) = s.get("/").await;
    assert_eq!(st, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body.contains("brickie measure"));
}
