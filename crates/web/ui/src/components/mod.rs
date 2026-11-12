//! The page components, rendered on the server and hydrated in the browser.

mod app;
mod call;
mod canvas;
mod controls;
mod keys;
mod panels;
mod source;

pub use app::app;
pub use call::call;
pub use canvas::dash;
#[cfg(not(target_arch = "wasm32"))]
pub use source::source_view;
pub use source::{CW, LH, ice_col};
