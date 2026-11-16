//! `brickie check` on hand-built caches (no rust-analyzer needed): one pass and one violation
//! per rule type, nested brick trees, staleness, config includes and config errors.

mod config;
mod cycles_complexity;
mod deps;
mod nested;
mod scope;
mod short;
mod stale;
mod support;
mod text;
