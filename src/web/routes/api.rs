//! JSON endpoints that take the same params as the page.

use crate::web::{AppState, Failure, data};
use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use brickie_ui::Params;

pub async fn diagnostics(
    State(st): State<AppState>,
    Query(p): Query<Params>,
) -> Result<Response, Failure> {
    let l = st.load()?;
    let data = data::page(&l, &st.cache_file());
    let out: Vec<_> = data
        .diags
        .iter()
        .filter(|d| brickie_ui::diag_visible(d, d.accepted, &p))
        .collect();
    Ok(Json(out).into_response())
}

pub async fn functions(
    State(st): State<AppState>,
    Query(p): Query<Params>,
) -> Result<Response, Failure> {
    let l = st.load()?;
    let data = data::page(&l, &st.cache_file());
    let idx = brickie_ui::fn_rows(&data.functions, &p);
    let out: Vec<_> = idx
        .iter()
        .take(p.limit().unwrap_or(usize::MAX))
        .map(|&i| &data.functions[i])
        .collect();
    Ok(Json(out).into_response())
}

pub async fn graph(State(st): State<AppState>) -> Result<Response, Failure> {
    let l = st.load()?;
    Ok(Json(data::page(&l, &st.cache_file()).graph).into_response())
}
