//! brickie's engine: measure Rust workspaces (module dependency edges, function complexity),
//! cache the result, load the brick config and check it. The CLI and the webview drive it.
//! See docs/design.md.

pub mod cache;
pub mod check;
pub mod config;
pub mod diag;
pub mod measure;
pub mod review;
pub mod rules;
