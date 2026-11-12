//! brickie: measure Rust workspaces (module dependency edges, function complexity) and check
//! them against the project's own brick rules. See docs/design.md.

mod cli;

fn main() -> anyhow::Result<()> {
    let code = cli::run()?;
    std::process::exit(code);
}
