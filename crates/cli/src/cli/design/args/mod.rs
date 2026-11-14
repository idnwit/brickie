//! `brickie design` arguments. Bricks are named by path from the root (`web/api`), levels by
//! the composite brick that owns them (`--in web`; default: the root level).

use crate::cli::args::Format;
pub use brick::{BrickCmd, Kind};
use clap::{Args, Subcommand};
pub use other::{DepsCmd, ExportCmd, IncludeCmd, LimitCmd, Limits};
pub use rule::{RuleCmd, RuleSel};
use std::path::PathBuf;

mod brick;
mod other;
mod rule;

#[derive(Args)]
#[command(after_help = "Examples:
  brickie design show
  brickie design brick add store --kind crate --base 15
  brickie design rule add api store allow --reason \"reads data\" --base 16
  brickie design undo --base 17")]
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
    /// The version this edit is based on (from `design show`); refused when the design moved.
    #[arg(long, env = "BRICKIE_BASE")]
    pub base: u64,
    #[arg(long)]
    pub note: Option<String>,
    /// Validate fully in memory and print the diff and the new todos; write nothing.
    #[arg(long)]
    pub dry_run: bool,
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
    /// The design at the head: version, bricks, rules, limits and the todo list.
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
    /// Revert the file changes of version V (default: the head) as a new version.
    Undo {
        v: Option<u64>,
        #[arg(long, env = "BRICKIE_BASE")]
        base: u64,
    },
    /// Apply the ops read from stdin (one per line) as ONE version.
    Apply {
        #[command(flatten)]
        w: Write,
    },
    #[command(flatten)]
    Edit(Edit),
}

#[derive(Subcommand)]
pub enum Edit {
    /// Add, change, rename, move, remove, group or ungroup bricks.
    #[command(subcommand)]
    Brick(BrickCmd),
    /// Add, change or remove dependency rules.
    #[command(subcommand)]
    Rule(RuleCmd),
    /// Set or remove the exports of a composite.
    #[command(subcommand)]
    Export(ExportCmd),
    /// Add, change or remove complexity limits.
    #[command(subcommand)]
    Limit(LimitCmd),
    /// Set the defaults for edges no rule matches.
    #[command(subcommand)]
    Deps(DepsCmd),
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
