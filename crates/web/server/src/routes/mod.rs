//! HTTP routes: the page, the code viewer, the JSON endpoints, review and the wasm assets.

mod api;
mod assets;
mod index;
mod interface;
mod review;

pub use assets::wasm_embedded;

use super::AppState;
use crate::code::CodeProvider;
use axum::Router;
use axum::routing::{get, post};
use brickie_web_api::{
    CODE_PATH, Diagnostics, EdgeUses, Endpoint, Functions, Graph, Interface, PAGE_PATH, Review,
};
use std::sync::Arc;

pub fn router(state: AppState) -> Router {
    let code = codeview_host::router(Arc::new(CodeProvider::new(state.config())));
    Router::new()
        .nest_service(CODE_PATH.trim_end_matches('/'), code)
        .route(PAGE_PATH, get(index::index))
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
