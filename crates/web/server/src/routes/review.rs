//! `POST /review`: accept or unaccept a finding in `brickie.review.toml`.

use super::guard::Guard;
use crate::{AppState, Failure};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use brickie_check::check as review;
use brickie_web_api::{ReqOf, ResOf, Review};

/// Accept (`reason`) or unaccept (`reason: null`) one finding in `brickie.review.toml`.
pub async fn review(
    State(st): State<AppState>,
    _: Guard,
    Json(req): Json<ReqOf<Review>>,
) -> Result<Json<ResOf<Review>>, Failure> {
    let bad = |m: &str| Failure(StatusCode::BAD_REQUEST, m.to_string());
    if req.id.len() != 8 || !req.id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad("invalid finding id"));
    }
    let reason = req.reason.as_deref().map(str::trim);
    if reason == Some("") {
        return Err(bad("a reason is required to accept a finding"));
    }
    if let Some(r) = reason {
        // Accepting needs a finding to exist; unaccepting a vanished one must stay possible.
        let l = st.load()?;
        if !l.diagnostics.iter().any(|d| d.id == req.id) {
            return Err(Failure(
                StatusCode::NOT_FOUND,
                format!("no finding {}", req.id),
            ));
        }
        let _ = r;
    }
    {
        let _g = st.review_lock.lock().unwrap_or_else(|e| e.into_inner());
        review::set_review(&st.config, &req.id, reason)
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))?;
    }
    Ok(Json(brickie_web_api::ReviewReply {
        id: req.id,
        accepted: reason.is_some(),
        reason: reason.map(str::to_string),
    }))
}
