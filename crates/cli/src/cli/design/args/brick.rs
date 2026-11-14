//! Brick commands.

use super::Write;
use clap::{Args, Subcommand};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Kind {
    Crate,
    Module,
    Group,
}

/// Fields of a brick; lists are comma-separated.
#[derive(Args)]
pub struct BrickFields {
    #[arg(long, value_delimiter = ',')]
    pub members: Option<Vec<String>>,
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
    #[arg(long)]
    pub desc: Option<String>,
    /// Folder of a composite (its brickie.toml), relative to the level file.
    #[arg(long)]
    pub path: Option<String>,
}

#[derive(Subcommand)]
pub enum BrickCmd {
    /// Add a brick to a level; a crate defaults to `path:crates/<name>/`, a group to its own
    /// folder. Missing code is scaffolded.
    Add {
        name: String,
        #[arg(long = "in")]
        level: Option<String>,
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        #[command(flatten)]
        f: BrickFields,
        #[command(flatten)]
        w: Write,
    },
    /// Change the fields given; an empty `--desc` or `--path` removes it.
    Set {
        brick: String,
        #[command(flatten)]
        f: BrickFields,
        #[command(flatten)]
        w: Write,
    },
    /// Rename a brick; rules and `via` entries that name it follow.
    Rename {
        brick: String,
        to: String,
        #[command(flatten)]
        w: Write,
    },
    /// Move a brick into another composite (`--into root`: the root level).
    Move {
        brick: String,
        #[arg(long)]
        into: String,
        #[command(flatten)]
        w: Write,
    },
    /// Remove a brick and the rules that name it.
    Rm {
        brick: String,
        #[command(flatten)]
        w: Write,
    },
    /// Create composite NAME next to the children and move them into it.
    Group {
        name: String,
        #[arg(required = true)]
        children: Vec<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Lift a composite's children (and their rules) into its level.
    Ungroup {
        brick: String,
        #[command(flatten)]
        w: Write,
    },
}
