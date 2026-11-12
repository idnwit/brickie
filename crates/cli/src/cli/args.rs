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
    Check {
        #[arg(long, default_value = "brickie.toml")]
        config: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
        color: ColorArg,
    },
    /// Serve the reviewer webview.
    View {
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

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}
