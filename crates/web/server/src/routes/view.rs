//! The page data of a request: the working design (the draft when one exists, else the
//! published files) checked against the measured code, in view mode and Design mode alike.
//! `edit=1` only adds the [`DesignState`](brickie_web_api::DesignState).
//!
//! The one fallback: when the working design cannot be checked against code (no cache yet),
//! Design mode shows the design files alone, with a notice in the state. View mode has nothing
//! to show then and fails.

use crate::{AppState, Failure, data};
use axum::http::StatusCode;
use brickie_change::Design;
use brickie_web_api::{DesignState, LevelView, PageData, Params};

pub fn fail(e: anyhow::Error) -> Failure {
    Failure(StatusCode::CONFLICT, format!("{e:#}"))
}

pub fn design(st: &AppState) -> Result<Design, Failure> {
    Design::at(&st.config()).map_err(fail)
}

/// The working design's page data and, when it fell back to the design files alone, why.
fn working(st: &AppState, d: &Design) -> Result<(PageData, Option<String>), Failure> {
    let first = brickie_change::working::load(&st.config());
    let (l, notice) = match first {
        Ok((l, _)) => (l, None),
        Err(e) => {
            let l = d.load_working().map_err(fail)?;
            let why = format!("showing the design only, no measured code: {e:#}");
            (l, Some(why))
        }
    };
    Ok((data::page(&l, &st.cache_file()), notice))
}

/// The page data of Design mode: the working design with its [`DesignState`].
fn edit_page(st: &AppState) -> Result<(PageData, DesignState), Failure> {
    let d = design(st)?;
    let mut state = data::design::state(&d).map_err(fail)?;
    let (data, notice) = working(st, &d)?;
    state.notice = notice;
    Ok((data, state))
}

/// Every level of the working design and the state it was read with (Design mode).
pub fn edit_levels(st: &AppState) -> Result<(Vec<LevelView>, DesignState), Failure> {
    let (data, state) = edit_page(st)?;
    Ok((data.levels, state))
}

/// The whole page data for `p`.
pub fn page(st: &AppState, p: &Params) -> Result<PageData, Failure> {
    if !p.editing() {
        return Ok(data::page(&st.load()?, &st.cache_file()));
    }
    let (mut data, state) = edit_page(st)?;
    data.design = Some(state);
    Ok(data)
}
