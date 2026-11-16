//! brickie webview components.
//!
//! The same components run in two places: on the server, where [`render_page`] renders them to
//! HTML with hydration keys, and in the browser, where the `hydrate` export (feature `hydrate`,
//! wasm32 only) attaches signals to that HTML. See `docs/design/serve.md`, "Layout port".

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

/// Id of the element the app renders into, and of the script holding its JSON data.
pub const APP_ID: &str = "app";
pub const DATA_ID: &str = "brickie-data";

#[cfg(not(target_arch = "wasm32"))]
pub use components::render_page;
