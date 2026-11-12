//! `POST /api/interface` and `POST /api/edge-uses`: a brick's declared interface and the
//! reference sites behind an edge. An unknown brick path is a 400.

use crate::{AppState, Failure, data};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use brickie_web_api::{EdgeUses, Interface, ReqOf, ResOf};

fn bad(e: anyhow::Error) -> Failure {
    Failure(StatusCode::BAD_REQUEST, format!("{e:#}"))
}

pub async fn interface(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<Interface>>,
) -> Result<Json<ResOf<Interface>>, Failure> {
    let l = st.load()?;
    data::interface(&l, &req.brick).map(Json).map_err(bad)
}

pub async fn edge_uses(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<EdgeUses>>,
) -> Result<Json<ResOf<EdgeUses>>, Failure> {
    let l = st.load()?;
    data::edge_uses(&l, &req.from, &req.to)
        .map(Json)
        .map_err(bad)
}
