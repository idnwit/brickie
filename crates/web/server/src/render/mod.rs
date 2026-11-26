//! HTML rendering: the shell around the viewer's server-rendered page, and the docs page.
//!
//! This brick turns data into HTML strings and nothing else: it reads no files, loads no cache
//! and makes no requests (the `routes` brick calls it with data built by the `data` brick). The
//! viewer itself is drawn by `brickie-ui`; this brick only wraps it.
//!
//! # Usage
//!
//! [`index_html`] wraps the viewer output of a [`PageData`] in a full page; pass the wasm
//! bundle URLs to make it interactive, or `None` for the read-only page. [`docs_html`] renders a
//! [`DocsPage`] as docs.rs-style HTML with no JavaScript.
//!
//! ```ignore
//! // Crate-private: shown for orientation only.
//! let html = render::index_html(&data, &Params::default(), Some((js_url, wasm_url)));
//! let docs = render::docs_html(&docs_page);
//! assert!(docs.contains("<title>"));
//! ```
//!
//! # Key functions
//!
//! - [`index_html`]: the viewer page; embeds the page data as JSON and the wasm loader.
//! - [`docs_html`]: a brick's docs page (sidebar, root-module doc, children, items with uses).
//!
//! # Invariants
//!
//! - Text from the workspace (names, docs, signatures) is escaped; doc markdown is rendered with
//!   raw HTML disabled.
//! - Embedded JSON escapes `<`, so a `</script>` in the data cannot end the script element.

mod docs;

pub use docs::{docs_html, readme_html};

use brickie_web_api::{PageData, Params};

/// JSON embedded in a `<script>`: `<` is escaped so `</script>` cannot occur.
fn embed_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v)
        .unwrap_or_default()
        .replace('<', "\\u003c")
}

const SHELL_HEAD: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">";

/// The page: SSR output in `#app`, the data it hydrates from, and the wasm loader at `bundle`
/// (the JS glue and wasm URLs) when the bundle is embedded.
pub fn index_html(data: &PageData, params: &Params, bundle: Option<(String, String)>) -> String {
    let body = brickie_ui::render_page(data, params);
    let script = if let Some((js, wasm)) = bundle {
        format!(
            "<script type=\"module\">import init, {{hydrate}} from \"{js}\";\
await init({{module_or_path: \"{wasm}\"}});hydrate();</script>"
        )
    } else {
        String::new()
    };
    format!(
        "{SHELL_HEAD}<title>brickie</title><style>{css}</style></head><body><div id=\"{app}\">{body}</div>\
<script type=\"application/json\" id=\"{did}\">{json}</script>{script}</body></html>",
        css = brickie_ui::CSS,
        app = brickie_ui::APP_ID,
        did = brickie_ui::DATA_ID,
        json = embed_json(data),
    )
}
