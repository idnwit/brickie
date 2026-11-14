//! HTTP routes: the page, the code viewer, the JSON endpoints, review, the design editor's
//! endpoints and the measure job (writes behind `guard`) and the wasm assets.

mod api;
mod assets;
mod editor;
mod events;
mod guard;
mod index;
mod interface;
mod measure;
mod review;
mod view;

pub use assets::wasm_embedded;

use super::AppState;
use crate::code::CodeProvider;
use axum::Router;
use axum::routing::{get, post};
use brickie_web_api::*;
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
        .route(DesignGet::PATH, get(editor::state))
        .route(DesignEdit::PATH, post(editor::edit))
        .route(DesignMove::PATH, post(editor::move_brick))
        .route(DesignUndo::PATH, post(editor::undo))
        .route(DesignLinks::PATH, post(editor::links))
        .route(DesignLinkCheck::PATH, post(editor::link_check))
        .route(DesignLevels::PATH, post(editor::levels))
        .route(DesignLayout::PATH, post(editor::layout))
        .route(DesignRelayout::PATH, post(editor::relayout))
        .route(MeasureStatus::PATH, get(measure::status))
        .route(EVENTS_PATH, get(events::events))
        .route(MeasureStart::PATH, post(measure::start))
        .route("/static/ui/brickie_ui.js", get(assets::ui_js))
        .route("/static/ui/brickie_ui_bg.wasm", get(assets::ui_wasm))
        .with_state(state)
}
