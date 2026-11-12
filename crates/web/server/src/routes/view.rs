//! The page data of a request: the current design, or the open change's preview with
//! `change=1` (planned and proposed bricks marked). A preview that does not load falls back to
//! the current design; the change view carries the error.

use crate::{AppState, Failure, data};
use axum::http::StatusCode;
use brickie_change::Change;
use brickie_check::check::Loaded;
use brickie_web_api::{PageData, Params};

pub fn fail(e: anyhow::Error) -> Failure {
    Failure(StatusCode::CONFLICT, format!("{e:#}"))
}

/// The loaded check result for `p` and the open change's view.
pub fn loaded(
    st: &AppState,
    p: &Params,
) -> Result<(Loaded, Option<brickie_web_api::ChangeView>), Failure> {
    if !Change::is_open(&st.config) {
        return Ok((st.load()?, None));
    }
    let ch = Change::load(&st.config).map_err(fail)?;
    let view = data::change_view(&ch).map_err(fail)?;
    let preview = match p.editing() && view.error.is_none() {
        true => ch.preview(true).and_then(|pv| pv.check()).ok(),
        false => None,
    };
    let l = match preview {
        Some(l) => l,
        None => st.load()?,
    };
    Ok((l, Some(view)))
}

/// The whole page data for `p`.
pub fn page(st: &AppState, p: &Params) -> Result<PageData, Failure> {
    let (l, change) = loaded(st, p)?;
    let mut data = data::page(&l, &st.cache_file());
    if let Some(v) = &change
        && p.editing()
        && v.error.is_none()
    {
        data::mark(&mut data.levels, &l, v);
    }
    data.change = change;
    Ok(data)
}
