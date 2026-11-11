//! brickie: measure Rust workspaces (module dependency edges, function complexity) and check
//! them against the project's own brick rules. See docs/design.md.

mod cache;
mod check;
mod cli;
mod config;
mod diag;
mod measure;
mod review;
mod rules;
mod web;

fn main() -> anyhow::Result<()> {
    let code = cli::run()?;
    std::process::exit(code);
}
