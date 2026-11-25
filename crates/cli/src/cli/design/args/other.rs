//! Group, export, limit and include commands.

use super::Write;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum GroupCmd {
    /// Add a peer brick group to a level.
    Add {
        name: String,
        #[arg(long = "in")]
        level: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        #[arg(long)]
        desc: Option<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Change the tags or description of a group; empty values clear them.
    Set {
        group: String,
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        #[arg(long)]
        desc: Option<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Rename a group and every reference to it.
    Rename {
        group: String,
        to: String,
        #[command(flatten)]
        w: Write,
    },
    /// Remove an unused group.
    Rm {
        group: String,
        #[command(flatten)]
        w: Write,
    },
}

#[derive(Subcommand)]
pub enum ExportCmd {
    /// Export a child of a composite.
    Set {
        brick: String,
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
