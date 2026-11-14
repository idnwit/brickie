//! The design editor's endpoints (`/api/design/*`): the design at its head, writes as human
//! versions (edit, move, undo), link options and checks, levels, and canvas layout hints.
//! Every write passes the [`Guard`]. A stale base answers 409 with the `design/stale` text and
//! the head in the `x-brickie-head` header; any other refused write is a 409 with the engine's
//! message.

use super::guard::Guard;
use super::view::{design, edit_levels, fail};
use crate::{AppState, Failure, data};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use brickie_change::{By, Link, Outcome, Stale, brick_path};
use brickie_web_api::*;

type Res<E> = Result<Json<ResOf<E>>, Failure>;

/// A refused write: `design/stale` with the head in a header, or the engine's message.
pub struct Refused(anyhow::Error);

impl IntoResponse for Refused {
    fn into_response(self) -> Response {
        let head = self.0.downcast_ref::<Stale>().map(|s| s.head);
        let mut r = fail(self.0).into_response();
        if let Some(v) = head.and_then(|h| HeaderValue::from_str(&h.to_string()).ok()) {
            r.headers_mut().insert("x-brickie-head", v);
        }
        r
    }
}

impl From<Failure> for Refused {
    fn from(f: Failure) -> Refused {
        Refused(anyhow::anyhow!(f.1))
    }
}

/// The reply to a write: the design after it and every level.
fn reply(st: &AppState, out: Outcome) -> Result<Json<EditReply>, Refused> {
    let (levels, design) = edit_levels(st)?;
    Ok(Json(EditReply {
        changed: out.changed,
        design,
        levels,
    }))
}

pub async fn state(State(st): State<AppState>) -> Res<DesignGet> {
    Ok(Json(data::design::state(&design(&st)?).map_err(fail)?))
}

pub async fn edit(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<DesignEdit>>,
) -> Result<Json<EditReply>, Refused> {
    let out = design(&st)?
        .apply(req.base, By::Human, None, req.ops)
        .map_err(Refused)?;
    reply(&st, out)
}

pub async fn move_brick(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<DesignMove>>,
) -> Result<Json<EditReply>, Refused> {
    let d = design(&st)?;
    let op = d.move_op(&req.from, &req.into).map_err(Refused)?;
    let out = d
        .apply(req.base, By::Human, None, vec![op])
        .map_err(Refused)?;
    reply(&st, out)
}

pub async fn undo(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<DesignUndo>>,
) -> Result<Json<EditReply>, Refused> {
    let out = design(&st)?
        .undo(req.base, By::Human, req.v)
        .map_err(Refused)?;
    reply(&st, out)
}

pub async fn link_check(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<DesignLinkCheck>>,
) -> Res<DesignLinkCheck> {
    let link = Link {
        from: &req.from,
        to: &req.to,
        kinds: &req.kinds,
        scope: &req.scope,
        action: &req.action,
    };
    Ok(Json(
        design(&st)?.link_check(&req.level, &link).map_err(fail)?,
    ))
}

pub async fn links(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<DesignLinks>>,
) -> Res<DesignLinks> {
    Ok(Json(design(&st)?.link_options(&req.path).map_err(fail)?))
}

pub async fn levels(
    State(st): State<AppState>,
    Json(req): Json<ReqOf<DesignLevels>>,
) -> Res<DesignLevels> {
    let (mut out, _) = edit_levels(&st)?;
    if !req.paths.is_empty() {
        out.retain(|lv| req.paths.contains(&lv.path));
    }
    Ok(Json(out))
}

pub async fn layout(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<DesignLayout>>,
) -> Res<DesignLayout> {
    let path = brick_path(&req.brick);
    if path.is_empty() {
        return Err(Failure(StatusCode::BAD_REQUEST, "name a brick".into()));
    }
    let root = design(&st)?.root().to_path_buf();
    data::design::set_hint(&root, &req.brick, req.hint).map_err(fail)?;
    Ok(Json(()))
}

pub async fn relayout(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<DesignRelayout>>,
) -> Res<DesignRelayout> {
    let root = design(&st)?.root().to_path_buf();
    data::design::clear_level(&root, &req.level).map_err(fail)?;
    Ok(Json(()))
}
