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

pub(crate) use app::app;
use call::call;
use canvas::dash;
