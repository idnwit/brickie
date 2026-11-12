//! brickie webview components.
//!
//! The same components run in two places: on the server, where [`render_page`] renders them to
//! HTML with hydration keys, and in the browser, where the `hydrate` export (feature `hydrate`,
//! wasm32 only) attaches signals to that HTML. See `docs/design.md`, step 3.

mod components;
pub mod layout;
pub mod model;
mod platform;
pub mod state;
#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
mod wasm;

pub use components::{CW, LH, SourceLine, SourcePage, ice_col};
pub use model::*;

/// Stylesheet shared by the page and the source popup.
pub const CSS: &str = concat!(
    include_str!("../styles/theme.css"),
    include_str!("../styles/viewer.css"),
    include_str!("../styles/state.css"),
    include_str!("../styles/card.css"),
    include_str!("../styles/source.css"),
    include_str!("../styles/tree.css"),
);

/// Id of the element the app renders into, and of the script holding its JSON data.
pub const APP_ID: &str = "app";
pub const DATA_ID: &str = "brickie-data";

/// Render the app for `params` to HTML (children of `#app`, with `data-hk` hydration keys).
#[cfg(not(target_arch = "wasm32"))]
pub fn render_page(data: &PageData, params: &Params) -> String {
    let (data, params) = (data.clone(), params.clone());
    sycamore::render_to_string(move || components::app(data, params))
}

/// Render the source popup page body.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_source(page: &SourcePage) -> String {
    let page = page.clone();
    sycamore::render_to_string(move || components::source_view(page))
}
