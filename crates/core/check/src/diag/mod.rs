//! Findings, diagnostics and their text rendering.
//!
//! A rule describes a violation by implementing [`Finding`]; [`diagnostic`] assembles it into
//! the plain [`Diagnostic`] value of `brickie-model`. JSON output is that value's
//! serialisation, and the text printers ([`render_all`], [`render_short`]) and the webview are
//! consumers of the same value, so nothing here is formatted for later parsing. This brick
//! decides how findings look, not which ones exist.
//!
//! # Usage
//!
//! ```
//! use brickie_check::diag::{diagnostic, render_short, Finding, Note, RuleRef, Span};
//! use std::path::Path;
//!
//! struct TooLong;
//!
//! impl Finding for TooLong {
//!     fn code(&self) -> String { "complexity/lines".into() }
//!     fn subject(&self) -> String { "demo::run".into() }       // stable identity
//!     fn message(&self) -> String { "`run` has 90 lines (limit 60)".into() }
//!     fn spans(&self) -> Vec<Span> {
//!         vec![Span { file: "src/lib.rs".into(), line: 10, col: Some(8), len: Some(3), label: None }]
//!     }
//!     fn rule(&self) -> Option<RuleRef> { None }
//!     fn help(&self) -> Option<String> { Some("split it".into()) }
//! }
//!
//! let d = diagnostic(&TooLong);
//! assert_eq!(d.code, "complexity/lines");
//! assert_eq!(d.id.len(), 8);                 // hash of code and subject
//! let out = render_short(&[d], Path::new("."));
//! assert!(out.starts_with("error[complexity/lines]"));
//! assert!(out.contains("src/lib.rs:10:8"));
//! # let _ = Note { text: String::new(), span: None };
//! ```
//!
//! # Key items
//!
//! - [`Finding`]: what a rule implements; only the parts, [`diagnostic`] assembles them.
//! - [`diagnostic`] and [`finding_id`]: finding to [`Diagnostic`], and its 8-hex-digit id.
//! - [`render_all`]: cargo-style text with source snippets, then a summary line.
//! - [`render_short`]: one line per finding, for coding agents.
//! - [`Color`]: whether text output is colored (`Auto` follows `NO_COLOR` and the terminal).
//!
//! # Notes
//!
//! The id of a finding is stable across runs for the same code and subject. Paths under `root`
//! print relative to it. [`render_all`] reads source files under `root` for snippets and omits
//! the snippet when a file cannot be read.

mod model;
mod short;
mod text;

pub use model::*;
pub use short::render_short;
pub use text::{Color, render_all};
