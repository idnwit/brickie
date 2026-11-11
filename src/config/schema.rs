//! Configuration types: bricks, dependency rules, complexity rules.

use super::Sel;
use serde::{Deserialize, Serialize};

/// A named group of modules. `members` are package names (all targets of the package) or
/// module ids such as `my-engine::a::b` (the module and everything below it).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brick {
    pub name: String,
    pub members: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warn,
}

/// Which code a rule looks at; the default is production code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    Prod,
    Test,
    All,
}

impl Scope {
    pub fn matches(self, scope: &str) -> bool {
        match self {
            Scope::All => true,
            Scope::Prod => scope == "prod",
            Scope::Test => scope == "test",
        }
    }
}

pub const KINDS: [&str; 3] = ["uses", "implements", "reexports"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepRule {
    pub from: Sel,
    pub to: Sel,
    /// Empty means all kinds.
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub scope: Scope,
    pub action: Action,
    pub reason: Option<String>,
    pub help: Option<String>,
    /// `file:line` of the rule header, filled in while loading.
    #[serde(skip)]
    pub loc: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexityRule {
    #[serde(default)]
    pub bricks: Sel,
    pub cognitive: Option<u32>,
    pub cyclomatic: Option<u32>,
    pub nesting: Option<u32>,
    pub paths: Option<u64>,
    pub params: Option<u32>,
    pub lines: Option<u32>,
    #[serde(default = "default_level")]
    pub level: Level,
    #[serde(default)]
    pub scope: Scope,
    pub help: Option<String>,
    #[serde(skip)]
    pub loc: String,
}

fn default_level() -> Level {
    Level::Error
}

#[derive(Debug, Clone)]
pub struct Deps {
    /// For edges matching no rule.
    pub default: Action,
    /// `Deny` reports cycles between bricks.
    pub cycles: Action,
    pub rules: Vec<DepRule>,
}

impl Default for Deps {
    fn default() -> Self {
        Deps {
            default: Action::Allow,
            cycles: Action::Allow,
            rules: Vec::new(),
        }
    }
}

/// The merged configuration: `brickie.toml` plus everything it includes.
#[derive(Debug, Default)]
pub struct Config {
    pub bricks: Vec<Brick>,
    pub deps: Deps,
    pub complexity: Vec<ComplexityRule>,
}
