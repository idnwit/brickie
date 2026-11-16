//! brickie-check: load the brick config, evaluate the rules over the cache and render
//! diagnostics. `check` is the facade the CLI and the webview use.
//!
//! Folder bricks (see `brickie.toml`): `diag` and `config` <- `rules` / `interface` <- `check`.

pub mod check;
pub mod config;
pub mod diag;
pub mod interface;
mod rules;
