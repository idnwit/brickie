//! Rule and link commands (both have the same fields).

use super::Write;
use clap::{Args, Subcommand};

/// Which rule or link: `from` and `to` as written, narrowed by kinds and scope.
#[derive(Args)]
pub struct RuleSel {
    pub from: String,
    /// A rule: a brick, `tag:x` or `*`. A link: a sibling brick or `composite/child`.
    pub to: String,
    /// Repeat for several kinds: uses, implements, reexports (default: all).
    #[arg(long = "kind")]
    pub kinds: Vec<String>,
    /// prod, test or all (the default).
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long = "in")]
    pub level: Option<String>,
}

#[derive(Subcommand)]
pub enum RuleCmd {
    /// Add one: a rule allows links `from -> to`; a link declares a dependency.
    Add {
        #[command(flatten)]
        sel: RuleSel,
        /// Why it exists (required).
        #[arg(long)]
        reason: String,
        #[command(flatten)]
        w: Write,
    },
    /// Change the reason of one.
    Set {
        #[command(flatten)]
        sel: RuleSel,
        #[arg(long)]
        reason: Option<String>,
        #[command(flatten)]
        w: Write,
    },
    /// Remove one.
    Rm {
        #[command(flatten)]
        sel: RuleSel,
        #[command(flatten)]
        w: Write,
    },
}
