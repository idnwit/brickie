//! The code viewer host: an axum [`Router`] over a [`Provider`].
//!
//! It serves the viewer page, the embedded bundle (app wasm and Monaco) and the port endpoints,
//! calling the provider for data. It validates every path with [`codeview_port::valid_path`]
//! before the provider sees it. It does not define the data or the endpoints (that is
//! `codeview-port`) and does not build the UI; the bundle is embedded at build time by the `ui`
//! cargo feature.
//!
//! # Usage
//!
//! A host application implements [`Provider`] and mounts the router under a prefix:
//!
//! ```no_run
//! use codeview_host::router;
//! use codeview_port::{Band, FileText, Location, Mark, Pos, Provider, View};
//! use std::sync::Arc;
//!
//! struct Mine;
//!
//! impl Provider for Mine {
//!     fn views(&self) -> Vec<View> { vec![] }
//!     fn file(&self, _path: &str) -> Option<FileText> { None }
//!     fn marks(&self, _path: &str) -> Vec<Mark> { vec![] }
//!     fn bands(&self, _path: &str) -> Vec<Band> { vec![] }
//!     fn definition(&self, _path: &str, _pos: Pos) -> Vec<Location> { vec![] }
//!     fn references(&self, _path: &str, _pos: Pos) -> Vec<Location> { vec![] }
//! }
//!
//! let app = axum::Router::<()>::new().nest_service("/code", router(Arc::new(Mine)));
//! // serve `app` with axum::serve as usual; the viewer is then at /code/
//! # let _ = app;
//! ```
//!
//! # Key items
//!
//! * [`router`] builds the router for a provider.
//! * [`embedded`] tells whether the viewer bundle is part of this build.
//!
//! # Notes
//!
//! * Routes: `GET /` (the page), `GET /static/*` (the bundle) and `POST /api/*` (one route per
//!   endpoint of `codeview_port`).
//! * Without the `ui` feature, or without the wasm toolchain at build time, the page only says that
//!   the viewer is not built; the API endpoints still work.
//! * A file the provider does not know answers 404; an invalid path answers 400.

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
