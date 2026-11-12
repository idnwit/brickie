//! `GET /`: the server-rendered page.

use super::view;
use crate::{AppState, Failure, render};
use axum::extract::{Query, State};
use axum::response::Html;
use brickie_web_api::Params;

pub async fn index(
    State(st): State<AppState>,
    Query(params): Query<Params>,
) -> Result<Html<String>, Failure> {
    let data = view::page(&st, &params)?;
    Ok(Html(render::index_html(
        &data,
        &params,
        super::wasm_embedded(),
    )))
}
