//! The port between the webview server and the viewer: endpoint contracts and the data that
//! crosses them.
//!
//! Every JSON endpoint is a marker type that implements [`Endpoint`] and names its path, request
//! and response type, so the server mounts it and the browser calls it from one definition. The
//! page data ([`PageData`], [`LevelView`], [`GraphView`], [`DiagView`], [`FnRow`]) and the view
//! state in the URL ([`Params`]) live here too. The crate has no I/O and no brickie logic: it
//! builds nothing from the cache (the server does that) and draws nothing (the viewer does). It
//! depends only on serde and the design model, so it compiles to wasm.
//!
//! # Usage
//!
//! The viewer state is the URL query: parse it, change one field, and write it back with the
//! defaults omitted.
//!
//! ```
//! use brickie_web_api::{Endpoint, Graph, Params, docs_href};
//!
//! let p = Params::from_query("?lv=root/web&brick=api&fn_sort=cognitive");
//! assert_eq!(p.level_path(), "root/web");
//! assert_eq!(p.query(), "brick=api&lv=root/web");
//! assert_eq!(p.at_level("root").url(), "/");
//!
//! // Endpoint paths and links are plain functions and constants.
//! assert_eq!(Graph::PATH, "/api/graph");
//! assert_eq!(docs_href("root/web/server"), "/docs/web/server");
//! ```
//!
//! Requests and responses are serde types; the JSON shape is the contract:
//!
//! ```
//! use brickie_web_api::{Interface, InterfaceRequest, ReqOf};
//!
//! let body: ReqOf<Interface> = InterfaceRequest { brick: "root/web/api".into() };
//! assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"brick":"root/web/api"}"#);
//! ```
//!
//! # Endpoints
//!
//! | Path | Method | Request | Response |
//! |------|--------|---------|----------|
//! | [`PAGE_PATH`] `/` | GET | [`Params`] (query) | HTML page |
//! | [`DOCS_PATH`] `/docs/<brick>` | GET | none | HTML docs page ([`DocsPage`]) |
//! | [`CODE_PATH`] `/code/` | GET | `path`, `line`, `col` | code viewer |
//! | [`Graph`] `/api/graph` | GET | [`Params`] | `Vec<`[`LevelView`]`>` |
//! | [`Diagnostics`] `/api/diagnostics` | GET | [`Params`] | `Vec<`[`DiagView`]`>` |
//! | [`Functions`] `/api/functions` | GET | [`Params`] | `Vec<`[`FnRow`]`>` |
//! | [`Interface`] `/api/interface` | POST | [`InterfaceRequest`] | [`InterfaceView`] |
//! | [`EdgeUses`] `/api/edge-uses` | POST | [`EdgeUsesRequest`] | `Vec<`[`EdgeItemView`]`>` |
//! | [`MeasureStatus`], [`MeasureStart`] `/api/measure` | GET, POST | none | [`MeasureJob`] |
//! | [`EVENTS_PATH`] `/api/events` | GET | none | Server-Sent Events of [`ServerEvent`] |
//! | [`DesignGet`] `/api/design` and the `/api/design/*` writes | GET, POST | see each type | [`DesignState`], [`EditReply`] |
//!
//! # Key types and functions
//!
//! - [`Params`]: the URL state (level, selection, filters, mode).
//! - [`PageData`]: everything the page needs, embedded in the HTML as JSON.
//! - [`LevelView`] and [`GraphView`]: one level of the brick tree with its bricks and edges, no
//!   geometry (layout runs in the viewer crate).
//! - [`diag_visible_in`], [`fn_rows`], [`brick_scores`]: pure filters and scores both sides share.
//! - [`code_href`], [`docs_href`], [`code_brick_href`]: links into the code viewer and docs.
//!
//! # Invariants
//!
//! Everything here is pure data and pure functions: no clock, no file system, no network.
//! `Params` defaults are the empty string, and [`Params::query`] omits them, so equal views have
//! equal URLs. A contract change fails to build on both the server and the viewer.

/// The design's op, version and sheet types (`brickie_model::design`).
pub use brickie_model::design;

mod endpoint;
mod model;

pub use endpoint::*;
pub use model::*;
