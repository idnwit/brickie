//! The port endpoints: `POST` JSON in, JSON out, the provider run off the async threads. Paths
//! are checked with [`valid_path`] before the provider sees them.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::MethodRouter;
use axum::{Json, routing};
use codeview_port::*;
use std::sync::Arc;

type Answer<T> = Result<T, StatusCode>;

/// A `POST` route answering endpoint `E` with `f`.
pub fn post<E, P>(f: fn(&P, E::Req) -> Answer<E::Res>) -> MethodRouter<Arc<P>>
where
    E: Endpoint + 'static,
    E::Req: Send + 'static,
    E::Res: Send + 'static,
    P: Provider,
{
    routing::post(
        move |State(p): State<Arc<P>>, Json(req): Json<E::Req>| async move {
            tokio::task::spawn_blocking(move || f(&p, req))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .map(Json)
        },
    )
}

fn checked(path: &str) -> Answer<&str> {
    if valid_path(path) {
        Ok(path)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub fn views<P: Provider>(p: &P, _: ()) -> Answer<Vec<View>> {
    Ok(p.views())
}

pub fn file<P: Provider>(p: &P, r: PathReq) -> Answer<FileText> {
    p.file(checked(&r.path)?).ok_or(StatusCode::NOT_FOUND)
}

pub fn marks<P: Provider>(p: &P, r: PathReq) -> Answer<Vec<Mark>> {
    Ok(p.marks(checked(&r.path)?))
}

pub fn bands<P: Provider>(p: &P, r: PathReq) -> Answer<Vec<Band>> {
    Ok(p.bands(checked(&r.path)?))
}

pub fn definition<P: Provider>(p: &P, r: AtReq) -> Answer<Vec<Location>> {
    Ok(p.definition(checked(&r.path)?, r.pos))
}

pub fn references<P: Provider>(p: &P, r: AtReq) -> Answer<Vec<Location>> {
    Ok(p.references(checked(&r.path)?, r.pos))
}
