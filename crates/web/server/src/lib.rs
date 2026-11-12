//! `brickie serve`: the reviewer webview and the measure job.
//!
//! axum serves a page rendered by sycamore (`brickie-ui`) with hydration keys, JSON endpoints
//! and the code viewer (`/code/`). Every GET depends only on its query params, the cache and the config,
//! all read per request. The browser bundle (wasm) is optional: without it the page is a
//! read-only SSR page whose filters are plain links. See docs/design.md, step 3.
mod code;
mod data;
mod measure;
mod render;
mod routes;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use brickie_check::check::{self, Loaded};
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
    /// The address the server listens on, for the write guard (`routes::guard`).
    bind: Arc<Bind>,
    /// The measure job (`/api/measure`).
    measure: Arc<measure::Jobs>,
}

/// Host and port the server is bound to (`port` `None`: unknown, any port matches).
struct Bind {
    host: String,
    port: Option<u16>,
}

impl AppState {
    pub fn new(config: PathBuf) -> Self {
        AppState {
            config: Arc::new(config),
            review_lock: Arc::new(Mutex::new(())),
            bind: Arc::new(Bind {
                host: "127.0.0.1".into(),
                port: None,
            }),
            measure: Arc::new(measure::Jobs::new(measure::measure_runner())),
        }
    }

    /// The same state with another measure runner (tests).
    #[cfg(test)]
    fn with_runner(mut self, runner: measure::Runner) -> Self {
        self.measure = Arc::new(measure::Jobs::new(runner));
        self
    }

    /// The same state for a server bound to `host:port`.
    pub fn bound(mut self, host: &str, port: u16) -> Self {
        self.bind = Arc::new(Bind {
            host: host.to_string(),
            port: Some(port),
        });
        self
    }

    fn load(&self) -> Result<Loaded, Failure> {
        check::load(&self.config)
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))
    }

    /// The config file.
    fn config(&self) -> PathBuf {
        self.config.as_ref().clone()
    }

    fn cache_file(&self) -> PathBuf {
        brickie_check::check::config_dir(&self.config).join(".brickie/cache.json")
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
    let state = AppState::new(opts.config.clone()).bound(&opts.bind, opts.port);
    // Fail early on a missing cache or config instead of on the first request.
    check::load(&opts.config)?;
    let addr = format!("{}:{}", opts.bind, opts.port);
    let rt = tokio::runtime::Runtime::new().context("start tokio")?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("bind {addr}"))?;
        eprintln!(
            "brickie: serving http://{addr}/ ({})",
            if routes::wasm_embedded() {
                "interactive"
            } else {
                "read-only SSR: no wasm bundle embedded"
            }
        );
        axum::serve(listener, router(state)).await.context("serve")
    })
}
