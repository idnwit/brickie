//! Rule commands.

use super::Write;
use clap::{Args, Subcommand};

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
