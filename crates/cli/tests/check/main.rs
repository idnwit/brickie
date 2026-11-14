//! `brickie check` on hand-built caches (no rust-analyzer needed): one pass and one violation
//! per rule type, nested brick trees, staleness, config includes, review acceptance and config
//! errors.

mod config;
mod cycles_complexity;
mod deps;
mod links;
mod nested;
mod scope;
mod stale;
mod support;
mod text;
