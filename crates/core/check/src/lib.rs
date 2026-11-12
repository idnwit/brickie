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
mod rules;
