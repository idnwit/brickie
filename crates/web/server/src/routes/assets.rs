//! The wasm bundle built by `scripts/build-ui.sh`, embedded when it exists at compile time.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

#[cfg(ui_wasm)]
const UI_JS: Option<&str> = Some(include_str!("../../../../../assets/ui/brickie_ui.js"));
#[cfg(ui_wasm)]
const UI_WASM: Option<&[u8]> = Some(include_bytes!(
    "../../../../../assets/ui/brickie_ui_bg.wasm"
));
#[cfg(not(ui_wasm))]
const UI_JS: Option<&str> = None;
#[cfg(not(ui_wasm))]
const UI_WASM: Option<&[u8]> = None;

/// Whether the interactive bundle is embedded.
pub fn wasm_embedded() -> bool {
    UI_WASM.is_some()
}

pub async fn ui_js() -> Response {
    match UI_JS {
        Some(js) => (
            [
                (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
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
                (header::CACHE_CONTROL, "no-cache"),
            ],
            w,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
