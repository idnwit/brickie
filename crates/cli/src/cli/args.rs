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
        /// Check the design of the open change (accepted and proposed ops) instead of the
        /// config files.
        #[arg(long)]
        change: bool,
    },
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
    /// The open design change: without a subcommand, show it (status, semantic diff, what is
    /// new since this reader's last read).
    Change(ChangeArgs),
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

#[derive(clap::Args)]
pub struct ChangeArgs {
    #[arg(long, default_value = "brickie.toml", global = true)]
    pub config: PathBuf,
    #[arg(long, value_enum, default_value_t = Format::Text, global = true)]
    pub format: Format,
    /// Who is reading or writing: the reader cursor of `show`, the author of `propose`.
    #[arg(long = "as", value_enum, default_value_t = Who::Agent, global = true)]
    pub who: Who,
    /// Show the entries after this seq; does not move the reader cursor.
    #[arg(long)]
    pub since: Option<u64>,
    /// List concrete tasks: planned bricks and edges, drift findings, rules without a reason,
    /// proposed, rejected and conflicting ops.
    #[arg(long)]
    pub todo: bool,
    #[command(subcommand)]
    pub action: Option<ChangeCmd>,
}

#[derive(Subcommand)]
pub enum ChangeCmd {
    /// Open a change (one at a time): snapshot the config tree.
    Open {
        #[arg(long)]
        intent: String,
    },
    /// Record an op read as JSON from stdin (proposed when `--as agent`).
    Propose {
        /// Read the op as JSON from stdin (the only input format).
        #[arg(long)]
        json: bool,
        #[arg(long)]
        note: Option<String>,
    },
    /// Accept a proposed (or rejected) op, as the human reviewer.
    Accept {
        seq: u64,
        #[arg(long)]
        note: Option<String>,
    },
    /// Reject an op, as the human reviewer; later ops that depend on it become conflicts.
    Reject {
        seq: u64,
        #[arg(long)]
        note: Option<String>,
    },
    /// Take back a human op (undo; default: the latest one). The log keeps it, rejected.
    Retract { seq: Option<u64> },
    /// Re-record a conflicting op as a human op against the current design; the original is
    /// rejected.
    Reapply { seq: u64 },
    /// Write the config files and scaffold new bricks, then close the change.
    Publish {
        /// Print the plan (files to write, create or edit; blockers) without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete the change; the config files are untouched.
    Discard,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Who {
    Human,
    Agent,
}
