//! Command line definition (clap derive).

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "brickie", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Measure the workspace and write the cache.
    Measure {
        #[arg(long, default_value = "brickie.toml")]
        config: PathBuf,
        /// Use an existing SCIP index instead of running rust-analyzer.
        #[arg(long)]
        scip: Option<PathBuf>,
        /// Workspace directory (defaults to the config's directory).
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
    /// Check the cached measurements against the rules.
    Check(CheckArgs),
    /// Print a brick's declared interface: each item's signature, doc and uses (by brick, with
    /// locations); unused items are marked. With `--from A`, every reference site from brick A
    /// into the brick instead (the code behind the edge A -> brick).
    Interface {
        /// Brick path, such as `root/web/api` or `web/api`.
        brick: String,
        #[arg(long, default_value = "brickie.toml")]
        config: PathBuf,
        #[arg(long)]
        json: bool,
        /// Using brick (path): list the sites of the edge from it.
        #[arg(long)]
        from: Option<String>,
    },
    /// Read and edit the design (the brickie.toml tree), versioned. Every edit is written at
    /// once and needs `--base N`, the version it was based on.
    Design(Box<super::design::DesignArgs>),
    /// Serve the reviewer webview; it can also run measure (the Measure button).
    Serve {
        #[arg(long, default_value = "brickie.toml")]
        config: PathBuf,
        #[arg(long, default_value_t = 7881)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
    /// Print the guide for coding agents.
    #[command(name = "llms.txt")]
    LlmsTxt,
}

/// Options of `brickie check`.
#[derive(clap::Args)]
pub struct CheckArgs {
    #[arg(long, default_value = "brickie.toml")]
    pub config: PathBuf,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,
    /// Show only findings owned by this brick (path from the root, such as `web/api`) or
    /// its descendants; findings without an owner always show. Repeatable.
    #[arg(long = "brick", value_name = "PATH")]
    pub bricks: Vec<String>,
    /// Hide warnings with this code (repeatable or comma separated); errors still show.
    #[arg(long, value_name = "CODE", value_delimiter = ',')]
    pub skip: Vec<String>,
    /// Run measure first when the cache is missing, unreadable or has stale findings.
    #[arg(long)]
    pub measure: bool,
    /// Write `.brickie/baseline.json` from the current findings (the exit code is as usual).
    #[arg(long, conflicts_with = "new")]
    pub save_baseline: bool,
    /// Hide the findings in the saved baseline unless they got worse; fail when the rule
    /// files changed since it was saved.
    #[arg(long)]
    pub new: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Text,
    Json,
    /// One line per finding, for coding agents.
    Short,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}
