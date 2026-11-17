//! `brickie design` arguments. Bricks are named by path from the root (`web/api`), levels by
//! the composite brick that owns them (`--in web`; default: the root level).

use crate::cli::args::Format;
pub use brick::{BrickCmd, Kind};
use clap::{Args, Subcommand};
pub use other::{ExportCmd, IncludeCmd, LimitCmd, Limits};
pub use rule::{RuleCmd, RuleSel};
use std::path::PathBuf;

mod brick;
mod other;
mod rule;

#[derive(Args)]
#[command(after_help = "Examples:
  brickie design show
  brickie design brick add store --kind crate --base 15
  brickie design rule add api store --reason \"api may read data\" --base 16
  brickie design link add api store --kind uses --reason \"reads orders\" --base 17
  brickie design undo --base 17
  brickie design check
  brickie design publish --base 17")]
pub struct DesignArgs {
    #[arg(long, default_value = "brickie.toml", global = true)]
    pub config: PathBuf,
    #[arg(long, value_enum, default_value_t = Format::Text, global = true)]
    pub format: Format,
    #[command(subcommand)]
    pub cmd: Cmd,
}

// Every write: the version it is based on, and why.
#[derive(Args)]
pub struct Write {
    /// The version this edit is based on: the draft head (from `design show`), or the published
    /// head when there is no draft yet; refused when it moved.
    #[arg(long, env = "BRICKIE_BASE")]
    pub base: u64,
    #[arg(long)]
    pub note: Option<String>,
    /// Validate fully in memory and print the diff and the new todos; write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Set by `brick add --scaffold` and `apply --scaffold`; never a flag of its own here.
    #[arg(skip)]
    pub scaffold: bool,
}

/// One design op per line, the syntax of the write subcommands without `brickie design`.
#[derive(clap::Parser)]
#[command(
    no_binary_name = true,
    disable_help_flag = true,
    disable_help_subcommand = true
)]
pub struct Line {
    #[command(subcommand)]
    pub edit: Edit,
}

// One short-lived value per run: boxing the flattened `Edit` buys nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum Cmd {
    /// The working design (the draft, else the published one) from its files alone: version,
    /// bricks, rules, limits and the config todo list. Never reads the code.
    Show {
        /// Only the versions after N, with what each changed.
        #[arg(long)]
        since: Option<u64>,
        /// Only the level owned by composite P (a path such as `web`) and its subtree.
        #[arg(long = "in")]
        level: Option<String>,
    },
    /// Every kept version, oldest first.
    Log,
    /// What changed from version A to B (default: the head).
    Diff { a: u64, b: Option<u64> },
    /// Revert the file changes of version V (default: the working head) as a new version of
    /// its track (--base: that track's head).
    Undo {
        v: Option<u64>,
        #[arg(long, env = "BRICKIE_BASE")]
        base: u64,
    },
    /// Apply the ops read from stdin (one per line) as ONE version.
    Apply {
        /// Also scaffold the missing code of every brick the lines add (default: design
        /// files only).
        #[arg(long)]
        scaffold: bool,
        #[command(flatten)]
        w: Write,
    },
    /// Compare the code with the working design (the draft, else the published one): the items
    /// that block a publish, plus the findings that do not. Exit 4 when blocked.
    Check,
    /// Copy the draft into the real files as one published version, once the code matches its
    /// structure (`check` lists what blocks).
    Publish {
        /// The draft head.
        #[arg(long, env = "BRICKIE_BASE")]
        base: u64,
        #[arg(long)]
        note: Option<String>,
    },
    /// Replay the draft's ops onto the new published head (after the published design moved).
    Rebase {
        /// The draft head.
        #[arg(long, env = "BRICKIE_BASE")]
        base: u64,
    },
    /// Drop the draft; the published design stays as it is.
    Discard {
        /// The draft head.
        #[arg(long, env = "BRICKIE_BASE")]
        base: u64,
    },
    #[command(flatten)]
    Edit(Edit),
}

#[derive(Subcommand)]
pub enum Edit {
    /// Add, change, rename, move, remove, group or ungroup bricks.
    #[command(subcommand)]
    Brick(BrickCmd),
    /// Add, change or remove rules: the links a level allows.
    #[command(subcommand)]
    Rule(RuleCmd),
    /// Add, change or remove links: the declared dependencies between sibling bricks.
    #[command(subcommand)]
    Link(RuleCmd),
    /// Set or remove the exports of a composite.
    #[command(subcommand)]
    Export(ExportCmd),
    /// Add, change or remove complexity limits.
    #[command(subcommand)]
    Limit(LimitCmd),
    /// Add or remove included preset files.
    #[command(subcommand)]
    Include(IncludeCmd),
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    fn walk(cmd: &clap::Command, path: &str, bad: &mut Vec<String>) {
        for sub in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
            let p = format!("{path} {}", sub.get_name());
            if sub
                .get_about()
                .is_none_or(|a| a.to_string().trim().is_empty())
            {
                bad.push(p.clone());
            }
            walk(sub, &p, bad);
        }
    }

    #[test]
    fn every_design_subcommand_has_help() {
        let root = crate::cli::args::Cli::command();
        let design = root.find_subcommand("design").expect("design command");
        let mut bad = Vec::new();
        walk(design, "design", &mut bad);
        assert!(bad.is_empty(), "no help: {bad:?}");
    }
}
