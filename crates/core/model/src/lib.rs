//! brickie's shared vocabulary: plain, wasm-safe data types used by the engine and the viewer.
//!
//! This brick is responsible for the shapes that cross crate boundaries: the structured
//! [`Diagnostic`] every finding renders to, the stable diagnostic [`codes`], the per-function
//! decision tree ([`FnTree`]) and the design edit types in [`design`] ([`design::Op`],
//! [`design::Version`], and the views the editor reads). It is deliberately free of I/O, of
//! rule logic and of any dependency beyond `serde`, so the browser build can use it as is. It
//! does not decide whether something is a violation (that is `brickie-check`) and it does not
//! write files (that is `brickie-change`).
//!
//! # Usage
//!
//! Build a diagnostic by hand, and look codes up by their stable string:
//!
//! ```
//! use brickie_model::{codes, Diagnostic, Span};
//!
//! let d = Diagnostic {
//!     id: "d1".into(),
//!     code: codes::DEPS_UNLINKED.into(),
//!     message: "root/web uses root/core without a link".into(),
//!     span: Some(Span {
//!         file: "crates/web/src/lib.rs".into(),
//!         line: 12,
//!         col: Some(5),
//!         len: Some(9),
//!         label: None,
//!     }),
//!     spans: vec![],
//!     rule: None,
//!     help: Some("add a [[link]] or remove the use".into()),
//!     notes: vec![],
//!     bricks: vec!["web".into(), "core".into()],
//!     brick_level: Some("root".into()),
//!     edges: vec![],
//! };
//! assert!(codes::all().contains(&d.code));
//! ```
//!
//! A design edit is an [`design::Op`]; the JSON form is what `brickie design` reads:
//!
//! ```
//! use brickie_model::design::Op;
//!
//! let op: Op = serde_json::from_str(
//!     r#"{"op": "add_export", "file": "brickie.toml", "brick": "api"}"#,
//! )
//! .unwrap();
//! assert_eq!(op.name(), "add_export");
//! assert_eq!(op.file(), "brickie.toml");
//! ```
//!
//! # Key items
//!
//! | Item | Meaning |
//! | --- | --- |
//! | [`Diagnostic`], [`Span`], [`Note`], [`RuleRef`], [`DiagEdge`] | One finding and its locations |
//! | [`codes`] | Every diagnostic code string, and [`codes::all`] |
//! | [`FnTree`], [`TreeNode`] | A function's decision tree with its complexity numbers |
//! | [`design::Op`] | One design edit (`add_brick`, `add_link`, ...) |
//! | [`design::Version`], [`design::Track`], [`design::By`] | One entry of the design history |
//! | [`design::DesignView`] | The design tree the viewer shows |
//! | [`design::Gate`], [`design::Blocker`] | What still blocks publishing the draft |
//!
//! # Notes
//!
//! Types derive `Serialize` (and `Deserialize` where they are read back); optional fields are
//! omitted from JSON when empty, so a field added later does not break older files.

pub mod codes;
pub mod design;
mod diagnostic;
mod tree;

pub use diagnostic::{DiagEdge, Diagnostic, Note, RuleRef, Span};
pub use tree::{FnTree, TreeNode};
