//! `GET /docs/<brick path>` and `GET /readme/<brick path>`: a brick's documentation page and
//! rendered `README.md`. Pass `?embed=1` to omit the sidebar for in-panel iframe embedding.

use crate::{AppState, Failure, data, render};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct EmbedQuery {
    #[serde(default)]
    embed: String,
}

impl EmbedQuery {
    fn is_embed(&self) -> bool {
        self.embed == "1" || self.embed == "true"
    }
}

pub async fn root(
    State(st): State<AppState>,
    Query(q): Query<EmbedQuery>,
) -> Result<Html<String>, Failure> {
    page(&st, "", q.is_embed())
}

pub async fn brick(
    State(st): State<AppState>,
    Path(brick): Path<String>,
    Query(q): Query<EmbedQuery>,
) -> Result<Html<String>, Failure> {
    page(&st, &brick, q.is_embed())
}

pub async fn readme_root(
    State(st): State<AppState>,
    Query(q): Query<EmbedQuery>,
) -> Result<Html<String>, Failure> {
    readme_page(&st, "", q.is_embed())
}

pub async fn readme_brick(
    State(st): State<AppState>,
    Path(brick): Path<String>,
    Query(q): Query<EmbedQuery>,
) -> Result<Html<String>, Failure> {
    readme_page(&st, &brick, q.is_embed())
}

fn page(st: &AppState, brick: &str, embed: bool) -> Result<Html<String>, Failure> {
    let l = st.load()?;
    let p = data::docs(&l, brick).map_err(|e| Failure(StatusCode::NOT_FOUND, format!("{e:#}")))?;
    Ok(Html(render::docs_html(&p, embed)))
}

fn readme_page(st: &AppState, brick: &str, embed: bool) -> Result<Html<String>, Failure> {
    let l = st.load()?;
    let (level, md) =
        data::readme(&l, brick).map_err(|e| Failure(StatusCode::NOT_FOUND, format!("{e:#}")))?;
    Ok(Html(render::readme_html(&level, &md, embed)))
}
