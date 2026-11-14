//! `brickie design` arguments. Bricks are named by path from the root (`web/api`), levels by
//! the composite brick that owns them (`--in web`; default: the root level).

use crate::cli::args::Format;
use clap::{Args, Subcommand};
use std::path::PathBuf;

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
}

#[derive(Subcommand)]
pub enum Cmd {
    /// The design at the head: version, bricks, rules, limits and the todo list.
    Show {
        /// Only the versions after N, with what each changed.
        #[arg(long)]
        since: Option<u64>,
    },
    /// Every kept version, oldest first.
    Log,
    /// What changed from version A to B (default: the head).
    Diff { a: u64, b: Option<u64> },
    /// Revert the file changes of version V (default: the head) as a new version.
    Undo {
        v: Option<u64>,
        #[command(flatten)]
        w: Write,
    },
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

/// Which rule: `from` and `to` as written, narrowed by kinds and scope.
#[derive(Args)]
pub struct RuleSel {
    pub from: String,
    pub to: String,
    /// Repeat for several kinds: uses, implements, reexports.
    #[arg(long = "kind")]
    pub kinds: Vec<String>,
    /// prod (the default in the file), test or all.
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long = "in")]
    pub level: Option<String>,
}

#[derive(Args)]
pub struct RuleFields {
    #[arg(long)]
    pub reason: Option<String>,
    /// Exported children of the `to` composite the rule applies to (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub via: Option<Vec<String>>,
    /// The fix shown with a finding (the rule's `help`).
    #[arg(long)]
    pub fix: Option<String>,
}

#[derive(Subcommand)]
pub enum RuleCmd {
    /// Add a rule: `from -> to` is allowed or denied.
    Add {
        #[command(flatten)]
        sel: RuleSel,
        #[arg(value_parser = ["allow", "deny"])]
        action: String,
        #[command(flatten)]
        f: RuleFields,
        #[command(flatten)]
        w: Write,
    },
    /// Change the action, reason, via or fix of a rule (an empty value removes it).
    Set {
        #[command(flatten)]
        sel: RuleSel,
        #[arg(long, value_parser = ["allow", "deny"])]
        action: Option<String>,
        #[command(flatten)]
        f: RuleFields,
        #[command(flatten)]
        w: Write,
    },
    /// Remove a rule.
    Rm {
        #[command(flatten)]
        sel: RuleSel,
        #[command(flatten)]
        w: Write,
    },
}

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
