//! The design editor's endpoints (`/api/change/*`): the open change's state, recording human
//! ops, reviews, retract and re-apply (answered with the touched preview levels), link options
//! and checks, preview levels, canvas positions, publish and discard. Every write passes
//! the [`Guard`]; a refused change operation is a 409 with the engine's message.

use super::guard::Guard;
use super::view::fail;
use crate::{AppState, Failure, data};
use axum::Json;
use axum::extract::State;
use brickie_change::{By, Change, Link};
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
    let ch = open(&st)?;
    let files = data::op_files(&req.op);
    let rec = ch.record(By::Human, req.op, req.note).map_err(fail)?;
    Ok(Json(data::reply(&ch, rec, &files).map_err(fail)?))
}

pub async fn move_brick(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeMove>>,
) -> Res<ChangeMove> {
    let ch = open(&st)?;
    let op = ch.move_op(&req.from, &req.into).map_err(fail)?;
    let files = data::op_files(&op);
    let rec = ch.record(By::Human, op, req.note).map_err(fail)?;
    Ok(Json(data::reply(&ch, rec, &files).map_err(fail)?))
}

pub async fn review(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeReview>>,
) -> Res<ChangeReview> {
    let note = req.note.filter(|n| !n.trim().is_empty());
    let ch = open(&st)?;
    let entry = ch
        .review(By::Human, req.seq, req.verdict, note)
        .map_err(fail)?;
    let files = data::seq_files(&ch, entry.target).map_err(fail)?;
    Ok(Json(data::reply(&ch, entry, &files).map_err(fail)?))
}

pub async fn retract(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeRetract>>,
) -> Res<ChangeRetract> {
    let ch = open(&st)?;
    let entry = ch.retract(req.seq).map_err(fail)?;
    let files = data::seq_files(&ch, entry.target).map_err(fail)?;
    Ok(Json(data::reply(&ch, entry, &files).map_err(fail)?))
}

pub async fn reapply(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<ChangeReapply>>,
) -> Res<ChangeReapply> {
    let ch = open(&st)?;
    let rec = ch.reapply(req.seq).map_err(fail)?;
    let files = data::op_files(&rec.op);
    Ok(Json(data::reply(&ch, rec, &files).map_err(fail)?))
}

pub async fn link_check(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<ChangeLinkCheck>>,
) -> Res<ChangeLinkCheck> {
    let link = Link {
        from: &req.from,
        to: &req.to,
        kinds: &req.kinds,
        scope: &req.scope,
        action: &req.action,
    };
    Ok(Json(
        open(&st)?.link_check(&req.level, &link).map_err(fail)?,
    ))
}

pub async fn levels(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<ChangeLevels>>,
) -> Res<ChangeLevels> {
    Ok(Json(data::levels(&open(&st)?, &req.paths).map_err(fail)?))
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
