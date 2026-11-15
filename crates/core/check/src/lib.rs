//! brickie-check: load the brick config, evaluate the rules over the cache, apply review
//! decisions and render diagnostics. `check` is the facade the CLI and the webview use.
//!
//! Folder bricks (see `brickie.toml`): `diag` <- `config` <- `rules` / `interface` <- `check`,
//! plus `review`.

pub mod check;
pub mod config;
pub mod diag;
pub mod interface;
mod review;
/// Reads `brickie.review.toml` next to a config; used by the docs test to load examples.
pub use review::read as read_review;
mod rules;
