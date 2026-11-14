//! The page data of a request: the design as checked. In edit mode (`edit=1`) the design is
//! loaded as a draft (bricks without code yet do not fail it), marked with planned bricks and
//! edges, and the page carries the [`DesignState`](brickie_web_api::DesignState).

use crate::{AppState, Failure, data};
use axum::http::StatusCode;
use brickie_change::Design;
use brickie_check::check::{self, Loaded};
use brickie_check::config;
use brickie_web_api::{DesignState, LevelView, PageData, Params};

pub fn fail(e: anyhow::Error) -> Failure {
    Failure(StatusCode::CONFLICT, format!("{e:#}"))
}

pub fn design(st: &AppState) -> Result<Design, Failure> {
    Design::at(&st.config()).map_err(fail)
}

/// The design loaded as a draft, with its state.
fn draft(st: &AppState) -> Result<(Loaded, DesignState), Failure> {
    let state = data::design::state(&design(st)?).map_err(fail)?;
    let l = check::load_draft_from(&config::Disk, &st.config()).map_err(fail)?;
    Ok((l, state))
}

/// Every level of the design in edit mode, marked, and the state it was read with.
pub fn edit_levels(st: &AppState) -> Result<(Vec<LevelView>, DesignState), Failure> {
    let (l, state) = draft(st)?;
    let mut levels = data::graph::levels(&l);
    data::design::mark(&mut levels, &l, &state);
    Ok((levels, state))
}

/// The whole page data for `p`.
pub fn page(st: &AppState, p: &Params) -> Result<PageData, Failure> {
    if !p.editing() {
        return Ok(data::page(&st.load()?, &st.cache_file()));
    }
    let (l, state) = draft(st)?;
    let mut data = data::page(&l, &st.cache_file());
    data::design::mark(&mut data.levels, &l, &state);
    data.design = Some(state);
    Ok(data)
}
