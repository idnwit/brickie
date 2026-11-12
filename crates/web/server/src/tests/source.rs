//! `/source`: highlight, tree, escaping and confinement.

use super::*;

#[tokio::test]
async fn source_highlights_and_confines() {
    let s = Site::new();
    let (st, html) = s
        .get("/source?file=crates/eng/src/x.rs&line=2&col=8&end_col=12")
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(html.contains("id=\"L2\""));
    // The function around the line gets its complexity summary (computed on demand).
    assert!(
        html.contains("class=\"tsum\"") && html.contains("busy \u{b7} paths 1"),
        "{html}"
    );
    assert!(
        !s.get("/source?file=crates/eng/src/x.rs&line=1")
            .await
            .1
            .contains("class=\"tsum\"")
    );
    assert!(html.contains("<mark"));
    assert!(html.contains(">busy</mark>"), "{html}");
    // Content is escaped.
    assert!(html.contains("&lt;b&gt;&amp;&lt;/b&gt;"));
    assert!(!html.contains("<b>&</b>"));
    // Traversal and absolute paths are rejected; files outside the cache are not served.
    for bad in [
        "../secret.txt",
        "crates/eng/../../../secret.txt",
        "/etc/passwd",
        "crates//eng/src/x.rs",
        "",
    ] {
        let (st, _) = s.get(&format!("/source?file={}", encode(bad))).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (st, body) = s.get("/source?file=crates/eng/src/other.rs").await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{body}");
    let (st, _) = s.get("/source?file=Cargo.toml").await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // A listed file that is a symlink out of the workspace is not served.
    #[cfg(unix)]
    {
        let (st, body) = s.get("/source?file=crates/eng/src/link.rs").await;
        assert_eq!(st, StatusCode::NOT_FOUND);
        assert!(!body.contains("top secret"));
    }
}

fn encode(s: &str) -> String {
    brickie_ui::encode(s)
}
