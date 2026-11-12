//! The design editor's endpoints (`/api/change/*`): the open change's state, recording human
//! ops and reviews, link options, canvas positions, publish and discard. Every write passes
//! the [`Guard`]; a refused change operation is a 409 with the engine's message.

use super::guard::Guard;
use super::view::fail;
use crate::{AppState, Failure, data};
use axum::Json;
use axum::extract::State;
use brickie_change::{By, Change};
use brickie_web_api::*;

type Res<E> = Result<Json<ResOf<E>>, Failure>;

fn open(st: &AppState) -> Result<Change, Failure> {
    Change::load(&st.config).map_err(fail)
}

pub async fn state(State(st): State<AppState>) -> Res<ChangeState> {
    if !Change::is_open(&st.config) {
        return Ok(Json(None));
    }
    let ch = open(&st)?;
    Ok(Json(Some(data::change_view(&ch).map_err(fail)?)))
}

pub async fn open_change(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeOpen>>,
) -> Res<ChangeOpen> {
    let ch = Change::open(&st.config, req.intent.trim()).map_err(fail)?;
    Ok(Json(data::change_view(&ch).map_err(fail)?))
}

pub async fn op(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeOp>>,
) -> Res<ChangeOp> {
    let rec = open(&st)?.record(By::Human, req.op, req.note);
    Ok(Json(rec.map_err(fail)?))
}

pub async fn move_brick(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeMove>>,
) -> Res<ChangeMove> {
    let ch = open(&st)?;
    let op = ch.move_op(&req.from, &req.into).map_err(fail)?;
    Ok(Json(ch.record(By::Human, op, req.note).map_err(fail)?))
}

pub async fn review(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeReview>>,
) -> Res<ChangeReview> {
    let note = req.note.filter(|n| !n.trim().is_empty());
    let entry = open(&st)?.review(By::Human, req.seq, req.verdict, note);
    Ok(Json(entry.map_err(fail)?))
}

pub async fn links(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<ChangeLinks>>,
) -> Res<ChangeLinks> {
    Ok(Json(open(&st)?.link_options(&req.path).map_err(fail)?))
}

pub async fn layout(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeLayout>>,
) -> Res<ChangeLayout> {
    open(&st)?.set_layout(&req.brick, req.hint).map_err(fail)?;
    Ok(Json(()))
}

pub async fn plan(State(st): State<AppState>) -> Res<ChangePublishPlan> {
    Ok(Json(open(&st)?.publish_plan().map_err(fail)?))
}

pub async fn publish(State(st): State<AppState>, _: Guard) -> Res<ChangePublish> {
    Ok(Json(open(&st)?.publish().map_err(fail)?))
}

pub async fn discard(State(st): State<AppState>, _: Guard) -> Res<ChangeDiscard> {
    open(&st)?.discard().map_err(fail)?;
    Ok(Json(()))
}
