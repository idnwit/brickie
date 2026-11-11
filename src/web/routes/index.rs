//! `GET /`: the server-rendered page.

use crate::web::{AppState, Failure, data, render};
use axum::extract::{Query, State};
use axum::response::Html;
use brickie_ui::Params;

pub async fn index(
    State(st): State<AppState>,
    Query(params): Query<Params>,
) -> Result<Html<String>, Failure> {
    let loaded = st.load()?;
    let data = data::page(&loaded, &st.cache_file());
    Ok(Html(render::index_html(
        &data,
        &params,
        super::wasm_embedded(),
    )))
}
