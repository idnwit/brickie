//! The code viewer UI: an explorer of the host's trees, tabs, Monaco (read-only, through the
//! small JS glue in `static/glue.js`), marks, and definition / reference navigation. It speaks
//! only the port: endpoints to the host server, messages to the host page.
//!
//! [`logic`] is plain Rust (tested natively); the browser part (`web`) renders on the client
//! (no server-side rendering). It compiles on every target so it is measured and linted, but
//! only runs in a browser.

pub mod logic;
mod web;
