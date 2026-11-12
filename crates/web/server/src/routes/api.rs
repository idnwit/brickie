//! JSON endpoints that take the same params as the page. Typed by the `brickie-web-api`
//! contract: `Diagnostics`, `Functions` and `Graph`.

use crate::{AppState, Failure, data};
use axum::Json;
use axum::extract::{Query, State};
use brickie_web_api::{Diagnostics, Functions, Graph, ReqOf, ResOf};

pub async fn diagnostics(
    State(st): State<AppState>,
    Query(p): Query<ReqOf<Diagnostics>>,
) -> Result<Json<ResOf<Diagnostics>>, Failure> {
    let l = st.load()?;
    let data = data::page(&l, &st.cache_file());
    let out = data
        .diags
        .into_iter()
        .filter(|d| brickie_web_api::diag_visible(d, d.accepted, &p))
        .collect();
    Ok(Json(out))
}

pub async fn functions(
    State(st): State<AppState>,
    Query(p): Query<ReqOf<Functions>>,
) -> Result<Json<ResOf<Functions>>, Failure> {
    let l = st.load()?;
    let data = data::page(&l, &st.cache_file());
    let idx = brickie_web_api::fn_rows(&data.functions, &p);
    let out = idx
        .iter()
        .take(p.limit().unwrap_or(usize::MAX))
        .map(|&i| data.functions[i].clone())
        .collect();
    Ok(Json(out))
}

/// `Graph` takes no input (`Req = ()`), so there is nothing to extract.
pub async fn graph(State(st): State<AppState>) -> Result<Json<ResOf<Graph>>, Failure> {
    let l = st.load()?;
    Ok(Json(data::page(&l, &st.cache_file()).levels))
}
