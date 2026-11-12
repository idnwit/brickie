//! HTML shells around the sycamore output: the main page and the source popup page.

mod source;

pub use source::source_html;

use brickie_ui::{PageData, Params};

/// JSON embedded in a `<script>`: `<` is escaped so `</script>` cannot occur.
fn embed_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v)
        .unwrap_or_default()
        .replace('<', "\\u003c")
}

const SHELL_HEAD: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">";

/// The page: SSR output in `#app`, the data it hydrates from, and the wasm loader when embedded.
pub fn index_html(data: &PageData, params: &Params, wasm: bool) -> String {
    let body = brickie_ui::render_page(data, params);
    let script = if wasm {
        "<script type=\"module\">import init, {hydrate} from \"/static/ui/brickie_ui.js\";\
await init({module_or_path: \"/static/ui/brickie_ui_bg.wasm\"});hydrate();</script>"
    } else {
        ""
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
