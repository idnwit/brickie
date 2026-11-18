//! Configuration types: one config file (a level of the brick tree), its bricks, exports,
//! rules, links and complexity rules.

use super::Sel;
use serde::Deserialize;
use std::path::PathBuf;

/// A named group of modules. `members` are package names (all targets of the package), module
/// prefixes such as `my-engine::a::b` (the module and everything below it), or `path:<dir>/` (crates whose manifest is
/// under the folder, relative to the declaring file). A brick with `path` is a composite: its
/// children are in `<path>/brickie.toml` (`sub`), and its members default to `path:<path>/`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brick {
    /// The brick's name, unique among its siblings.
    pub name: String,
    /// Which modules belong to the brick; see the type docs for the forms.
    #[serde(default)]
    pub members: Vec<String>,
    /// Labels that rules select with `tag:<tag>`.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The folder of the composite's own `brickie.toml`; `None` for a leaf.
    pub path: Option<String>,
    /// One-line description shown in the viewer; no effect on checks.
    pub desc: Option<String>,
    /// The brick acts like a library of its level: its siblings (and what they nest) use it freely.
    #[serde(default)]
    pub independent: bool,
    /// Canonical directory of the declaring file; `path:` members are relative to it.
    #[serde(skip)]
    pub base: PathBuf,
    /// The composite's own config, loaded from `<path>/brickie.toml`; `None` for a leaf.
    #[serde(skip)]
    pub sub: Option<Box<Config>>,
}

/// Which code a rule or link covers; the default is all code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Production code only.
    Prod,
    /// Test code only.
    Test,
    /// Production and test code.
    #[default]
    All,
}

impl Scope {
    /// `prod`, `test` or `all`, as written in `brickie.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Scope::Prod => "prod",
            Scope::Test => "test",
            Scope::All => "all",
        }
    }

    /// Whether this scope covers a concrete scope of code (`prod` or `test`).
    pub fn matches(self, scope: &str) -> bool {
        match self {
            Scope::All => true,
            Scope::Prod => scope == "prod",
            Scope::Test => scope == "test",
        }
    }
}

fn prod() -> Scope {
    Scope::Prod
}

/// The edge kinds: `uses` (call, type, constant), `implements` (`impl Trait for X` where the
/// trait lives elsewhere) and `reexports` (a `pub use`).
pub const KINDS: [&str; 3] = ["uses", "implements", "reexports"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// `[[rule]]`: a principle that allows links `from -> to` (a brick, `tag:x` or `*`). Nothing
/// is allowed unless a rule allows it.
pub struct Rule {
    /// The brick, `tag:x` or `*` the rule allows links from.
    pub from: Sel,
    /// The brick, `tag:x` or `*` the rule allows links to.
    pub to: Sel,
    /// Empty means all kinds.
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub scope: Scope,
    /// Why the rule exists; required.
    pub reason: String,
    /// `file:line` of the rule header, filled in while loading.
    #[serde(skip)]
    pub loc: String,
}

/// `[[link]]`: a declared dependency from a brick of this level to a sibling, or to an
/// exported child of a sibling composite (`to = "composite/child"`). A code edge is allowed
/// only through a link that some rule allows.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// The sibling brick that depends.
    pub from: String,
    /// The sibling brick depended on, or `composite/child` for an exported child.
    pub to: String,
    /// Empty means all kinds.
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub scope: Scope,
    /// Why the dependency exists; required.
    pub reason: String,
    /// `file:line` of the link header, filled in while loading.
    #[serde(skip)]
    pub loc: String,
}

impl Link {
    /// The sibling brick `to` names, and the child it enters through (`composite/child`).
    pub fn target(&self) -> (&str, Option<&str>) {
        match self.to.split_once('/') {
            Some((b, c)) => (b, Some(c)),
            None => (&self.to, None),
        }
    }
}

/// `[[complexity]]`: per-function limits for the bricks it selects; a metric left out is not
/// limited. Nested levels can only tighten what an outer level sets.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexityRule {
    /// The bricks the limits apply to; the default is all of them.
    #[serde(default)]
    pub bricks: Sel,
    pub cognitive: Option<u32>,
    pub cyclomatic: Option<u32>,
    pub nesting: Option<u32>,
    pub paths: Option<u64>,
    pub params: Option<u32>,
    pub lines: Option<u32>,
    /// Complexity limits look at production code unless they say otherwise.
    #[serde(default = "prod")]
    pub scope: Scope,
    #[serde(skip)]
    pub loc: String,
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
    pub exports: Vec<String>,
    /// `[[rule]]`: the principles links between this level's children must follow.
    pub rules: Vec<Rule>,
    /// `[[link]]`: the declared dependencies between this level's children.
    pub links: Vec<Link>,
    pub complexity: Vec<ComplexityRule>,
    /// `[measure]`; only the root level may set it.
    pub measure: super::MeasureCfg,
    /// Root only: `config/missing` issues of the whole tree (a brick whose `path` file does
    /// not exist stays a leaf).
    pub missing: Vec<super::Issue>,
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

    /// The brick at `path` (`[]` has none).
    pub fn brick_at(&self, path: &[String]) -> Option<&Brick> {
        let (name, parent) = path.split_last()?;
        self.node(parent)?.brick(name)
    }

    /// The direct child brick named `name`.
    pub fn brick(&self, name: &str) -> Option<&Brick> {
        self.bricks.iter().find(|b| b.name == name)
    }

    /// Whether child `name` is visible from outside this composite.
    pub fn exports(&self, name: &str) -> bool {
        self.exports.iter().any(|e| e == name)
    }
}

/// The level name of a path from the root: `root` followed by the names, `/`-separated.
pub fn level_name(path: &[String]) -> String {
    std::iter::once("root")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("/")
}
