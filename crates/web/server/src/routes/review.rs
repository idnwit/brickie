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
    // Accepting needs a finding to exist (its weight bounds the acceptance); unaccepting a
    // vanished one must stay possible.
    let accept = match reason {
        Some(r) => {
            let l = st.load()?;
            let weight = (l.weights.get(&req.id).copied())
                .filter(|_| l.diagnostics.iter().any(|d| d.id == req.id))
                .ok_or_else(|| Failure(StatusCode::NOT_FOUND, format!("no finding {}", req.id)))?;
            Some((r, weight))
        }
        None => None,
    };
    {
        let _g = st.review_lock.lock().unwrap_or_else(|e| e.into_inner());
        review::set_review(&st.config, &req.id, accept)
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))?;
    }
    Ok(Json(brickie_web_api::ReviewReply {
        id: req.id,
        accepted: reason.is_some(),
        reason: reason.map(str::to_string),
    }))
}
