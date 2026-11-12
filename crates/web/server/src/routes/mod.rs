//! HTTP routes: the page, the source popup, the JSON endpoints, review and the wasm assets.

mod api;
mod assets;
mod index;
mod interface;
mod review;
mod source;

pub use assets::wasm_embedded;

use super::AppState;
use axum::Router;
use axum::routing::{get, post};
use brickie_web_api::{
    Diagnostics, EdgeUses, Endpoint, Functions, Graph, Interface, PAGE_PATH, Review, SOURCE_PATH,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(PAGE_PATH, get(index::index))
        .route(SOURCE_PATH, get(source::source))
        .route(Diagnostics::PATH, get(api::diagnostics))
        .route(Functions::PATH, get(api::functions))
        .route(Graph::PATH, get(api::graph))
        .route(Review::PATH, post(review::review))
        .route(Interface::PATH, post(interface::interface))
        .route(EdgeUses::PATH, post(interface::edge_uses))
        .route("/static/ui/brickie_ui.js", get(assets::ui_js))
        .route("/static/ui/brickie_ui_bg.wasm", get(assets::ui_wasm))
        .with_state(state)
}
