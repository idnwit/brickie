//! Configuration types: one config file (a level of the brick tree), its bricks, exports,
//! dependency rules and complexity rules.

use super::Sel;
use serde::Deserialize;
use std::path::PathBuf;

/// A named group of modules. `members` are package names (all targets of the package), module
/// ids such as `my-engine::a::b` (the module and everything below it) or `path:<dir>/` (crates
/// whose manifest is under the folder, relative to the declaring file). A brick with `path` is
/// a composite: its children are in `<path>/brickie.toml` (`sub`), and its members default to
/// `path:<path>/`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brick {
    pub name: String,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub path: Option<String>,
    /// Canonical directory of the declaring file; `path:` members are relative to it.
    #[serde(skip)]
    pub base: PathBuf,
    /// The composite's own config, loaded from `<path>/brickie.toml`; `None` for a leaf.
    #[serde(skip)]
    pub sub: Option<Box<Config>>,
}

/// An entry of a composite's `exports`: a direct child visible from outside, for edges of
/// `scope` (`all` for a plain name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub brick: String,
    pub scope: Scope,
}

impl<'de> Deserialize<'de> for Export {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Scoped {
            brick: String,
            #[serde(default = "all")]
            scope: Scope,
        }
        fn all() -> Scope {
            Scope::All
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Name(String),
            Scoped(Scoped),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Name(brick) => Export {
                brick,
                scope: Scope::All,
            },
            Raw::Scoped(s) => Export {
                brick: s.brick,
                scope: s.scope,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
}

pub use brickie_model::Level;

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

/// One level of the brick tree: a `brickie.toml` plus everything it includes. The root config
/// is the root level; a composite brick's `sub` is the level below it.
#[derive(Debug, Default, Clone)]
pub struct Config {
    /// Names from the root to this level (`[]` for the root, `["web"]` below brick `web`).
    pub path: Vec<String>,
    /// The config file, as `file:line` locations print it.
    pub file: String,
    pub bricks: Vec<Brick>,
    /// Children visible from outside this composite (unused at the root).
    pub exports: Vec<Export>,
    pub deps: Deps,
    pub complexity: Vec<ComplexityRule>,
}

impl Config {
    /// `root`, `root/web`, `root/web/api`.
    pub fn level(&self) -> String {
        level_name(&self.path)
    }

    /// The level (composite config) at `path`; `[]` is this level.
    pub fn node(&self, path: &[String]) -> Option<&Config> {
        match path.split_first() {
            None => Some(self),
            Some((head, rest)) => self.brick(head)?.sub.as_deref()?.node(rest),
        }
    }

    /// The direct child brick named `name`.
    pub fn brick(&self, name: &str) -> Option<&Brick> {
        self.bricks.iter().find(|b| b.name == name)
    }

    /// Whether child `name` is exported for edges of `scope` (`prod` or `test`).
    pub fn exports_to(&self, name: &str, scope: &str) -> bool {
        self.exports
            .iter()
            .any(|e| e.brick == name && e.scope.matches(scope))
    }
}

/// The level name of a path from the root: `root` followed by the names, `/`-separated.
pub fn level_name(path: &[String]) -> String {
    std::iter::once("root")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("/")
}
