//! HTTP routes: the page, the code viewer, the JSON endpoints, review, the design editor's
//! change endpoints and the measure job (writes behind `guard`) and the wasm assets.

mod api;
mod assets;
mod editor;
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
        .route(ChangeState::PATH, get(editor::state))
        .route(ChangeOpen::PATH, post(editor::open_change))
        .route(ChangeOp::PATH, post(editor::op))
        .route(ChangeMove::PATH, post(editor::move_brick))
        .route(ChangeReview::PATH, post(editor::review))
        .route(ChangeRetract::PATH, post(editor::retract))
        .route(ChangeReapply::PATH, post(editor::reapply))
        .route(ChangeLinks::PATH, post(editor::links))
        .route(ChangeLinkCheck::PATH, post(editor::link_check))
        .route(ChangeLevels::PATH, post(editor::levels))
        .route(ChangeLayout::PATH, post(editor::layout))
        .route(ChangePublishPlan::PATH, get(editor::plan))
        .route(ChangePublish::PATH, post(editor::publish))
        .route(ChangeDiscard::PATH, post(editor::discard))
        .route(MeasureStatus::PATH, get(measure::status))
        .route(MeasureStart::PATH, post(measure::start))
        .route("/static/ui/brickie_ui.js", get(assets::ui_js))
        .route("/static/ui/brickie_ui_bg.wasm", get(assets::ui_wasm))
        .with_state(state)
}
