//! The page components, rendered on the server and hydrated in the browser.

mod app;
mod call;
mod canvas;
mod controls;
mod editor;
mod keys;
mod links;
mod panels;
mod popup;

use app::app;
use brickie_web_api::{PageData, Params};

/// Render the app for `params` to HTML (children of `#app`, with `data-hk` hydration keys).
#[cfg(not(target_arch = "wasm32"))]
pub fn render_page(data: &PageData, params: &Params) -> String {
    let (data, params) = (data.clone(), params.clone());
    sycamore::render_to_string(move || app(data, params))
}

/// Attach the app to the server-rendered `host`.
#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
pub(crate) fn hydrate_in(data: PageData, params: Params, host: &web_sys::Element) {
    sycamore::web::hydrate_in_scope(|| app(data, params), host);
}
use canvas::dash;
