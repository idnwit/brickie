//! The brickie viewer: Sycamore components that render on the server and hydrate in the browser.
//!
//! The page (brick canvas, findings list, function table, design editor) is written once. On the
//! server, [`render_page`] turns a [`PageData`](brickie_web_api::PageData) and the URL
//! [`Params`](brickie_web_api::Params) into HTML with hydration keys. In the browser the
//! `hydrate` export (feature `hydrate`, wasm32 only) reads the same data from the page and
//! attaches signals to that HTML. This crate fetches nothing at render time and holds no data of
//! its own: the server builds the data (`brickie-web`) and the contract lives in
//! `brickie-web-api`. Layout geometry comes from the [`layout`] crate.
//!
//! # Usage
//!
//! Server side, render the app body and wrap it in the HTML shell:
//!
//! ```no_run
//! use brickie_ui::{APP_ID, CSS, DATA_ID, render_page};
//! use brickie_web_api::{PageData, Params};
//!
//! fn body(data: &PageData) -> String {
//!     let params = Params::from_query("?lv=root/web&brick=api");
//!     let app = render_page(data, &params);
//!     format!(
//!         "<style>{CSS}</style><div id=\"{APP_ID}\">{app}</div>\
//!          <script id=\"{DATA_ID}\" type=\"application/json\">{}</script>",
//!         serde_json::to_string(data).unwrap_or_default()
//!     )
//! }
//! ```
//!
//! Browser side, the wasm bundle calls `hydrate()` (see `src/wasm.rs`), which hydrates the
//! element with id [`APP_ID`] from the JSON in the element with id [`DATA_ID`].
//!
//! # Key items
//!
//! - [`render_page`]: the app as an HTML string (native targets only).
//! - [`CSS`]: the page stylesheet, for the shell to inline.
//! - [`APP_ID`], [`DATA_ID`]: the ids the server and the browser agree on.
//! - [`state`]: the pure view state under the components (its own brick).
//! - `components` (private): the canvas, controls, panels, source popup and editor.
//!
//! # Invariants
//!
//! - The server render and the first browser render of the same data and params produce the same
//!   HTML, or hydration breaks; keep browser-only reads behind the platform wrappers.
//! - Every view change is a [`Params`](brickie_web_api::Params) change, so a link and a click
//!   reach the same state.

mod components;
pub use brickie_layout as layout;
mod platform;
pub mod state;
#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
mod wasm;

/// The page stylesheet.
pub const CSS: &str = concat!(
    include_str!("../styles/theme.css"),
    include_str!("../styles/viewer.css"),
    include_str!("../styles/state.css"),
    include_str!("../styles/card.css"),
    include_str!("../styles/editor.css"),
    include_str!("../styles/editor-canvas.css"),
);

/// Id of the element the app renders into.
pub const APP_ID: &str = "app";
/// Id of the `<script type="application/json">` element holding the page data.
pub const DATA_ID: &str = "brickie-data";

#[cfg(not(target_arch = "wasm32"))]
pub use components::render_page;
