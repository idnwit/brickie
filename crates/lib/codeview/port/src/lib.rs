//! The code viewer's contract: data types, the [`Provider`] trait, HTTP endpoints and iframe
//! messages.
//!
//! A host application implements [`Provider`] to feed the viewer with explorer trees, file texts,
//! marks, bands and navigation targets. The viewer UI (`codeview-app`) and the router
//! (`codeview-host`) both build only on this crate, so they never depend on each other or on the
//! host. This crate holds no behaviour beyond [`valid_path`]; it does not serve HTTP or draw UI.
//!
//! # Usage
//!
//! Implement [`Provider`] over your own data and hand it to `codeview_host::router`:
//!
//! ```
//! use codeview_port::{FileText, Band, Location, Mark, Pos, Provider, View};
//!
//! struct Notes;
//!
//! impl Provider for Notes {
//!     fn views(&self) -> Vec<View> {
//!         vec![View { name: "Folders".into(), roots: vec![] }]
//!     }
//!     fn file(&self, path: &str) -> Option<FileText> {
//!         (path == "a.rs").then(|| FileText {
//!             path: path.into(),
//!             text: "fn a() {}".into(),
//!             lang: "rust".into(),
//!             stale: false,
//!         })
//!     }
//!     fn marks(&self, _path: &str) -> Vec<Mark> { vec![] }
//!     fn bands(&self, _path: &str) -> Vec<Band> { vec![] }
//!     fn definition(&self, _path: &str, _pos: Pos) -> Vec<Location> { vec![] }
//!     fn references(&self, _path: &str, _pos: Pos) -> Vec<Location> { vec![] }
//! }
//!
//! assert!(Notes.file("a.rs").is_some());
//! ```
//!
//! The page that embeds the viewer in an iframe talks to it with [`ToViewer`] and [`ToHost`]
//! messages, JSON with a `type` tag:
//!
//! ```
//! use codeview_port::ToViewer;
//!
//! let json = serde_json::to_string(&ToViewer::Theme { dark: true }).unwrap();
//! assert_eq!(json, r#"{"type":"Theme","dark":true}"#);
//! ```
//!
//! # Key items
//!
//! * [`Provider`] is what a host implements.
//! * [`Endpoint`] with the marker types [`Views`], [`File`], [`Marks`], [`Bands`], [`Definition`]
//!   and [`References`] names each endpoint's path and request and response types; the request
//!   types are [`PathReq`] and [`AtReq`].
//! * [`View`], [`Node`], [`NodeKind`] and [`Badge`] describe explorer trees.
//! * [`FileText`], [`Mark`], [`Band`], [`Location`], [`Level`], [`Pos`] and [`Span`] describe
//!   file contents and annotations.
//! * [`ToViewer`] and [`ToHost`] are the iframe messages.
//! * [`valid_path`] tells which paths a provider may receive.
//!
//! # Notes
//!
//! * Every endpoint is a `POST` with a JSON body, answered with JSON.
//! * Positions are 1-based: line and byte column.
//! * Paths given to a provider are relative, with no `.`, `..`, empty, `\`, `:` or NUL parts; the
//!   host router rejects anything else before calling the provider.
//! * A provider is shared across requests: it must be `Send + Sync + 'static`.

mod endpoint;
mod message;
mod model;
#[cfg(test)]
mod tests;

pub use endpoint::*;
pub use message::{ToHost, ToViewer};
pub use model::*;

/// What a host application implements (server side) to feed the viewer. `path` is always a
/// relative path without `.` or `..` components (the host router rejects anything else).
pub trait Provider: Send + Sync + 'static {
    /// The explorer trees, in display order.
    fn views(&self) -> Vec<View>;
    /// The text of `path`, or `None` when the host has no such file (answered as 404).
    fn file(&self, path: &str) -> Option<FileText>;
    /// The marks (findings, highlights) of `path`.
    fn marks(&self, path: &str) -> Vec<Mark>;
    /// The gutter bands of `path`.
    fn bands(&self, path: &str) -> Vec<Band>;
    /// Where the symbol at `pos` in `path` is defined.
    fn definition(&self, path: &str, pos: Pos) -> Vec<Location>;
    /// Where the symbol at `pos` in `path` is used.
    fn references(&self, path: &str, pos: Pos) -> Vec<Location>;
}
