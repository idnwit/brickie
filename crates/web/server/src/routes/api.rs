//! JSON endpoints that take the same params as the page. Typed by the `brickie-web-api`
//! contract: `Diagnostics`, `Functions` and `Graph` (of the change preview with `change=1`).

use super::view;
use crate::{AppState, Failure};
use axum::Json;
use axum::extract::{Query, State};
use brickie_web_api::{Diagnostics, Functions, Graph, ReqOf, ResOf};

pub async fn diagnostics(
    State(st): State<AppState>,
    Query(p): Query<ReqOf<Diagnostics>>,
) -> Result<Json<ResOf<Diagnostics>>, Failure> {
    let data = view::page(&st, &p)?;
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
    let data = view::page(&st, &p)?;
    let idx = brickie_web_api::fn_rows(&data.functions, &p);
    let out = idx
        .iter()
        .take(p.limit().unwrap_or(usize::MAX))
        .map(|&i| data.functions[i].clone())
        .collect();
    Ok(Json(out))
}

pub async fn graph(
    State(st): State<AppState>,
    Query(p): Query<ReqOf<Graph>>,
) -> Result<Json<ResOf<Graph>>, Failure> {
    Ok(Json(view::page(&st, &p)?.levels))
}
