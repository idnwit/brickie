//! The code viewer UI: an explorer of the host's trees, tabs, Monaco and navigation.
//!
//! It shows the trees a host provides, opens files in tabs in a read-only Monaco editor (through a
//! small JavaScript glue in `static/glue.js`), draws marks and bands, and follows definition and
//! reference links. It speaks only the port: endpoints to the host server, messages to the host
//! page. A host application never depends on this crate; its compiled bundle is embedded as files
//! by `codeview-host`.
//!
//! The pure logic is in [`logic`] and is tested natively. The browser part (`web`) renders on the
//! client only (no server-side rendering); it compiles on every target so it is measured and
//! linted, but only runs in a browser.
//!
//! # Usage
//!
//! The logic helpers are plain functions over `codeview_port` data:
//!
//! ```
//! use codeview_app::logic::{close_tab, open_tab, quick};
//!
//! let tabs = open_tab(&[], "src/lib.rs");
//! let tabs = open_tab(&tabs, "src/main.rs");
//! let (tabs, next) = close_tab(&tabs, "src/main.rs");
//! assert_eq!(tabs, vec!["src/lib.rs".to_string()]);
//! assert_eq!(next.as_deref(), Some("src/lib.rs"));
//!
//! let paths = vec!["src/lib.rs".to_string(), "src/main.rs".to_string()];
//! assert_eq!(quick("main", &paths, 5), vec!["src/main.rs".to_string()]);
//! ```
//!
//! # Key items
//!
//! * [`logic::Query`] holds the URL params (`path`, `line`, `col`, `view`, `filter`).
//! * [`logic::scoped`], [`logic::files`] and [`logic::entry_file`] work on explorer trees.
//! * [`logic::quick`] is the quick-open matcher; [`logic::open_tab`] and [`logic::close_tab`] model
//!   the tab list.
//! * [`logic::tag_hue`] gives a tag a stable colour.
//!
//! # Notes
//!
//! * The wasm bundle needs the wasm32 target and `wasm-bindgen-cli` at build time (see
//!   `codeview-host`).

pub mod logic;
mod web;
