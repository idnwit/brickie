//! `GET /docs/...`: the brick documentation pages.

use super::*;

#[tokio::test]
async fn docs_page_shows_items_with_full_doc_and_source() {
    let s = Site::new();
    let (st, body) = s.get("/docs/engine").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(
        body.contains("<h2 id=\"functions\">Functions</h2>"),
        "{body}"
    );
    assert!(body.contains("busy()"));
    // The root module's inner doc, rendered like item docs.
    assert!(
        body.contains("<p>The <strong>engine</strong> &lt;i&gt;root&lt;/i&gt;.</p>"),
        "{body}"
    );
    // The whole doc comment, rendered as markdown with raw HTML escaped.
    assert!(body.contains("<p>Does busy.</p>"), "{body}");
    assert!(
        body.contains("<strong>busy</strong> &lt;b&gt;raw&lt;/b&gt;"),
        "{body}"
    );
    assert!(
        body.contains("/code/?path=crates/eng/src/x.rs&amp;line=2"),
        "{body}"
    );
    assert!(body.contains("No use outside the brick"));
    // The sidebar marks the current brick.
    assert!(
        body.contains("class=\"cur\" href=\"/docs/engine\""),
        "{body}"
    );
}

#[tokio::test]
async fn docs_root_lists_bricks_and_unknown_brick_is_404() {
    let s = Site::new();
    for uri in ["/docs", "/docs/"] {
        let (st, body) = s.get(uri).await;
        assert_eq!(st, StatusCode::OK, "{uri}: {body}");
        assert!(body.contains("<h2 id=\"children\">Bricks</h2>"), "{body}");
        assert!(body.contains("href=\"/docs/engine\""));
    }
    let (st, body) = s.get("/docs/nope").await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(body.contains("no brick"));
}

#[tokio::test]
async fn docs_embed_omits_sidebar_and_readme_renders_markdown_and_mermaid() {
    let s = Site::new();
    let (st, body) = s.get("/docs/engine?embed=1").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(body.contains("<body class=\"embed\">"), "{body}");
    assert!(!body.contains("<nav class=\"side\">"), "{body}");

    // Without a README.md, /readme/ returns 404.
    let (st, _) = s.get("/readme/").await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Add a README.md beside the root brickie.toml with a code block and a mermaid block.
    std::fs::write(
        s.dir.path().join("README.md"),
        "# Workspace\n\n```rust\nlet x = 1;\n```\n\n```mermaid\ngraph LR\n  A --> B\n```\n",
    )
    .unwrap();
    let (st, body) = s.get("/readme/").await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert!(body.contains("<h1>Workspace</h1>"), "{body}");
    assert!(body.contains("h-k\">let</span>"), "{body}");
    assert!(body.contains("<div class=\"mermaid\">"), "{body}");
    assert!(body.contains("mermaid.initialize"), "{body}");
}
