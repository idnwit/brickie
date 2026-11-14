//! Export, limit, deps and include commands.

use super::Write;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ExportCmd {
    /// Export a child of a composite (add it, or change its scope).
    Set {
        brick: String,
        #[arg(long)]
        scope: Option<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Stop exporting a child of a composite.
    Rm {
        brick: String,
        #[command(flatten)]
        w: Write,
    },
}

/// Complexity limits; `0` (or `""`) removes one in `limit set`.
#[derive(Args)]
pub struct Limits {
    #[arg(long)]
    pub cognitive: Option<u64>,
    #[arg(long)]
    pub cyclomatic: Option<u64>,
    #[arg(long)]
    pub nesting: Option<u64>,
    #[arg(long)]
    pub paths: Option<u64>,
    #[arg(long)]
    pub params: Option<u64>,
    #[arg(long)]
    pub lines: Option<u64>,
    /// error or warn.
    #[arg(long)]
    pub level: Option<String>,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long = "in")]
    pub within: Option<String>,
}

#[derive(Subcommand)]
pub enum LimitCmd {
    /// Add complexity limits for a selector (default `*`).
    Add {
        bricks: Option<String>,
        #[command(flatten)]
        l: Limits,
        #[command(flatten)]
        w: Write,
    },
    /// Change the given fields of a complexity limit.
    Set {
        bricks: Option<String>,
        #[command(flatten)]
        l: Limits,
        #[command(flatten)]
        w: Write,
    },
    /// Remove a complexity limit.
    Rm {
        bricks: Option<String>,
        #[arg(long = "in")]
        level: Option<String>,
        #[command(flatten)]
        w: Write,
    },
}

#[derive(Subcommand)]
pub enum DepsCmd {
    /// `deps.default` (edges no rule matches) and `deps.cycles`.
    Set {
        #[arg(long, value_parser = ["allow", "deny"])]
        default: Option<String>,
        #[arg(long, value_parser = ["allow", "deny"])]
        cycles: Option<String>,
        #[arg(long = "in")]
        level: Option<String>,
        #[command(flatten)]
        w: Write,
    },
}

#[derive(Subcommand)]
pub enum IncludeCmd {
    /// Include a preset file (relative to the level file).
    Add {
        path: String,
        #[arg(long = "in")]
        level: Option<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Stop including a file.
    Rm {
        path: String,
        #[arg(long = "in")]
        level: Option<String>,
        #[command(flatten)]
        w: Write,
    },
}
