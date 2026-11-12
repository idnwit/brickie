//! `brickie view`: the reviewer webview.
//!
//! axum serves a page rendered by sycamore (`brickie-ui`) with hydration keys, JSON endpoints
//! and the source popup. Every GET depends only on its query params, the cache and the config,
//! all read per request. The browser bundle (wasm) is optional: without it the page is a
//! read-only SSR page whose filters are plain links. See docs/design.md, step 3.
mod data;
mod render;
mod routes;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use brickie_core::check::{self, Loaded};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub use routes::router;

pub struct Options {
    pub config: PathBuf,
    pub bind: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<PathBuf>,
    /// Serialises writes to `brickie.review.toml`.
    review_lock: Arc<Mutex<()>>,
}

impl AppState {
    pub fn new(config: PathBuf) -> Self {
        AppState {
            config: Arc::new(config),
            review_lock: Arc::new(Mutex::new(())),
        }
    }

    fn load(&self) -> Result<Loaded, Failure> {
        check::load(&self.config)
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))
    }

    fn cache_file(&self) -> PathBuf {
        brickie_core::review::config_dir(&self.config).join(".brickie/cache.json")
    }
}

/// An error rendered as plain text with a status code.
pub struct Failure(StatusCode, String);

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (
            self.0,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            self.1,
        )
            .into_response()
    }
}

pub fn run(opts: &Options) -> Result<()> {
    let state = AppState::new(opts.config.clone());
    // Fail early on a missing cache or config instead of on the first request.
    check::load(&opts.config)?;
    let addr = format!("{}:{}", opts.bind, opts.port);
    let rt = tokio::runtime::Runtime::new().context("start tokio")?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("bind {addr}"))?;
        eprintln!(
            "brickie: view on http://{addr}/ ({})",
            if routes::wasm_embedded() {
                "interactive"
            } else {
                "read-only SSR: no wasm bundle embedded"
            }
        );
        axum::serve(listener, router(state)).await.context("serve")
    })
}
