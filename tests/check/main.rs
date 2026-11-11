//! `brickie check` on hand-built caches (no rust-analyzer needed): one pass and one violation
//! per rule type, staleness, config includes, review acceptance and config errors.

mod config;
mod cycles_complexity;
mod deps;
mod stale;
mod support;
mod text;
