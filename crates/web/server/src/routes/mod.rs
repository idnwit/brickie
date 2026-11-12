//! HTTP routes: the page, the source popup, the JSON endpoints, review and the wasm assets.

mod api;
mod assets;
mod index;
mod review;
mod source;

pub use assets::wasm_embedded;

use super::AppState;
use axum::Router;
use axum::routing::{get, post};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index::index))
        .route("/source", get(source::source))
        .route("/api/diagnostics", get(api::diagnostics))
        .route("/api/functions", get(api::functions))
        .route("/api/graph", get(api::graph))
        .route("/review", post(review::review))
        .route("/static/ui/brickie_ui.js", get(assets::ui_js))
        .route("/static/ui/brickie_ui_bg.wasm", get(assets::ui_wasm))
        .with_state(state)
}
