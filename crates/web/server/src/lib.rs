//! `brickie serve`: the reviewer webview server (axum) and its background jobs.
//!
//! The crate answers HTTP for one workspace: the server-rendered page, the docs page of each
//! brick, the code viewer, the JSON endpoints of the viewer, the design editor's writes, the
//! measure job and the embedded wasm bundle. It turns the cache and the config into the data of
//! [`brickie_web_api`] and leaves drawing to `brickie-ui`. It is not a measurer or a checker: it
//! calls `brickie-measure` and `brickie-check` and shares their results. It keeps no state of
//! its own beyond the measure job and the file watcher; every GET reads the config and the
//! cache again, so the page always shows what is on disk.
//!
//! # Usage
//!
//! The `brickie` binary calls [`run`], which loads the config once (to fail early), binds the
//! address and serves until killed. Tests and embedders can build the [`router`] instead.
//!
//! ```no_run
//! use brickie_web::{AppState, Options, router, run};
//!
//! // Serve `brickie.toml` on http://127.0.0.1:7881/ and block.
//! run(&Options {
//!     config: "brickie.toml".into(),
//!     bind: "127.0.0.1".into(),
//!     port: 7881,
//! })?;
//!
//! // Or take the axum router (for example to nest it in a test).
//! let app = router(AppState::new("brickie.toml".into()).bound("127.0.0.1", 7881));
//! # let _ = app;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Endpoints
//!
//! | Request | Answer |
//! |---------|--------|
//! | `GET /?<params>` | the page: SSR HTML plus the JSON it hydrates from |
//! | `GET /docs/` and `GET /docs/<brick>` | the docs page of a brick (404 for an unknown brick) |
//! | `GET /code/` | the code viewer, backed by the cache and the check result |
//! | `GET /api/graph`, `/api/diagnostics`, `/api/functions` | the page data under the same params |
//! | `POST /api/interface`, `POST /api/edge-uses` | a brick's interface; the sites behind an edge |
//! | `GET /api/events` | Server-Sent Events: design, cache, measure and config changes |
//! | `GET /api/measure`, `POST /api/measure` | the measure job's state; start it (single flight) |
//! | `GET /api/design`, `POST /api/design/{edit,move,undo,publish,discard}` | the design and its writes |
//! | `POST /api/design/{links,link-check,levels,layout,relayout}` | link options, checks, levels, canvas hints |
//! | `GET /static/ui/<hash>/...` | the embedded wasm bundle (404 without the `ui` feature) |
//!
//! The typed contract of each JSON endpoint is in `brickie-web-api`.
//!
//! # Key types and functions
//!
//! - [`run`]: bind, start the watcher and serve.
//! - [`router`]: the axum `Router` over an [`AppState`].
//! - [`Options`]: config path, bind address and port.
//! - [`AppState`]: the config path, the bind address for the write guard, the measure job and
//!   the event hub.
//!
//! # Invariants and errors
//!
//! - Every endpoint that writes files (`/api/design/*` writes and `POST /api/measure`) passes the
//!   write guard: the server must be bound to loopback and the request must come from the page
//!   it served; otherwise the answer is 403 and nothing is written.
//! - A design write based on an old version answers 409 with `design/stale` and the head in the
//!   `x-brickie-head` header; a refused write is a 409 with the engine's message.
//! - [`run`] fails when the config or cache cannot be loaded, or the port is taken. A request
//!   that cannot load them answers 500 as plain text.
//! - Without the wasm bundle the page is read-only SSR and its filters are plain links.
//!
//! Design notes are in the repository's `docs/design/serve.md`.
mod code;
mod data;
mod events;
mod measure;
mod render;
mod routes;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use brickie_check::check::Loaded;
use std::path::PathBuf;
use std::sync::Arc;

pub use routes::router;

/// What [`run`] needs: the config, and where to listen.
pub struct Options {
    /// The root `brickie.toml`.
    pub config: PathBuf,
    /// The address to bind. Only a loopback address allows writes (see the write guard).
    pub bind: String,
    pub port: u16,
}

/// The shared state of every request: where the config is, what the server is bound to, the
/// measure job and the file watcher. Cheap to clone (all parts are `Arc`s).
#[derive(Clone)]
pub struct AppState {
    config: Arc<PathBuf>,
    /// The address the server listens on, for the write guard (`routes::guard`).
    bind: Arc<Bind>,
    /// The measure job (`/api/measure`).
    measure: Arc<measure::Jobs>,
    /// The file watcher's state and the event channel (`/api/events`).
    events: Arc<events::Hub>,
}

/// Host and port the server is bound to (`port` `None`: unknown, any port matches).
struct Bind {
    host: String,
    port: Option<u16>,
}

impl AppState {
    /// State for the config at `config`, bound to `127.0.0.1` with an unknown port until
    /// [`AppState::bound`] says otherwise.
    pub fn new(config: PathBuf) -> Self {
        AppState {
            config: Arc::new(config),
            bind: Arc::new(Bind {
                host: "127.0.0.1".into(),
                port: None,
            }),
            measure: Arc::new(measure::Jobs::new(measure::measure_runner())),
            events: Arc::new(events::Hub::new()),
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
        brickie_change::working::load(&self.config)
            .map(|(l, _)| l)
            .map_err(|e| Failure(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))
    }

    /// The config file.
    fn config(&self) -> PathBuf {
        self.config.as_ref().clone()
    }

    /// The directory of the root config (where `.brickie/` lives).
    fn root(&self) -> PathBuf {
        brickie_check::check::config_dir(&self.config)
    }

    fn cache_file(&self) -> PathBuf {
        brickie_check::check::config_dir(&self.config).join(".brickie/cache.json")
    }
}

/// An error rendered as plain text with a status code.
pub(crate) struct Failure(StatusCode, String);

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

/// Serves `opts.config` until the process ends: loads the config and cache once (so a missing
/// one fails here, not on the first request), starts the file watcher, binds the address and
/// serves [`router`]. Prints the URL to stderr. Errors: unreadable config or cache, port in use.
pub fn run(opts: &Options) -> Result<()> {
    let state = AppState::new(opts.config.clone()).bound(&opts.bind, opts.port);
    // Fail early on a missing cache or config instead of on the first request.
    brickie_change::working::load(&opts.config)?;
    let addr = format!("{}:{}", opts.bind, opts.port);
    let rt = tokio::runtime::Runtime::new().context("start tokio")?;
    rt.block_on(async {
        events::spawn(state.clone());
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| {
                format!(
                    "bind {addr} (is it in use? pick another port with `brickie serve --port <N>`)"
                )
            })?;
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
