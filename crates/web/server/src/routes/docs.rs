//! `GET /docs/<brick path>`: a brick's documentation page; `/docs/` lists the top bricks. An
//! unknown brick is a 404.

use crate::{AppState, Failure, data, render};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Html;

pub async fn root(State(st): State<AppState>) -> Result<Html<String>, Failure> {
    page(&st, "")
}

pub async fn brick(
    State(st): State<AppState>,
    Path(brick): Path<String>,
) -> Result<Html<String>, Failure> {
    page(&st, &brick)
}

fn page(st: &AppState, brick: &str) -> Result<Html<String>, Failure> {
    let l = st.load()?;
    let p = data::docs(&l, brick).map_err(|e| Failure(StatusCode::NOT_FOUND, format!("{e:#}")))?;
    Ok(Html(render::docs_html(&p)))
}
