//! The wasm bundle built by `build.rs` into `OUT_DIR`, embedded when the build produced it.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

#[cfg(ui_wasm)]
const UI_JS: Option<&str> = Some(include_str!(concat!(env!("OUT_DIR"), "/ui/brickie_ui.js")));
#[cfg(ui_wasm)]
const UI_WASM: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/ui/brickie_ui_bg.wasm"
)));
#[cfg(not(ui_wasm))]
const UI_JS: Option<&str> = None;
#[cfg(not(ui_wasm))]
const UI_WASM: Option<&[u8]> = None;

/// The bundle's content hash; its URLs live under `/static/ui/<tag>/`, so they change with it.
const UI_TAG: &str = env!("BRICKIE_UI_TAG");

/// The URLs of the JS glue and the wasm, when the bundle is embedded.
pub(crate) fn bundle() -> Option<(String, String)> {
    wasm_embedded().then(ui_urls)
}

/// The URL of the JS glue, then of the wasm.
pub(super) fn ui_urls() -> (String, String) {
    let base = format!("/static/ui/{UI_TAG}");
    (
        format!("{base}/brickie_ui.js"),
        format!("{base}/brickie_ui_bg.wasm"),
    )
}

/// A hashed URL never changes content, so the browser may keep it.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Whether the interactive bundle is embedded.
pub(crate) fn wasm_embedded() -> bool {
    UI_WASM.is_some()
}

pub async fn ui_js() -> Response {
    match UI_JS {
        Some(js) => (
            [
                (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
                (header::CACHE_CONTROL, IMMUTABLE),
            ],
            js,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn ui_wasm() -> Response {
    match UI_WASM {
        Some(w) => (
            [
                (header::CONTENT_TYPE, "application/wasm"),
                (header::CACHE_CONTROL, IMMUTABLE),
            ],
            w,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
