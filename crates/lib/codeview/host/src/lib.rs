//! The code viewer host: an axum router over a [`Provider`]. It serves the viewer page, the
//! embedded bundle (app wasm, Monaco) and the port endpoints; a host application mounts it,
//! for example with `Router::nest_service("/code", codeview_host::router(provider))`.

mod api;
mod bundle;
#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::get;
use codeview_port::{Bands, Definition, Endpoint, File, Marks, Provider, References, Views};
use std::sync::Arc;

pub use bundle::embedded;

/// The router for `provider`: `GET /` (the page), `GET /static/*` (the bundle) and `POST
/// /api/*` (the endpoints of `codeview_port`).
pub fn router<P: Provider>(provider: Arc<P>) -> Router {
    Router::new()
        .route("/", get(bundle::index))
        .route("/static/{*file}", get(bundle::file))
        .route(&abs(Views::PATH), api::post::<Views, P>(api::views))
        .route(&abs(File::PATH), api::post::<File, P>(api::file))
        .route(&abs(Marks::PATH), api::post::<Marks, P>(api::marks))
        .route(&abs(Bands::PATH), api::post::<Bands, P>(api::bands))
        .route(
            &abs(Definition::PATH),
            api::post::<Definition, P>(api::definition),
        )
        .route(
            &abs(References::PATH),
            api::post::<References, P>(api::references),
        )
        .with_state(provider)
}

fn abs(path: &str) -> String {
    format!("/{path}")
}
