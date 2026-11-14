//! What the viewer's editor reads of the design besides the tree: one config file's own rows
//! ([`FileSheet`]), the links a brick may draw ([`LinkOption`]) and whether a new rule would
//! change anything ([`LinkCheck`]).

use super::{ComplexitySpec, ExportSpec, RuleSpec};
use serde::{Deserialize, Serialize};

/// The editable content of one physical config file, not merged with what it includes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSheet {
    /// `deps.default` / `deps.cycles` as written (`None`: not set in this file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycles: Option<String>,
    /// `[[deps.rule]]` in file order.
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    /// `[[complexity]]` in file order.
    #[serde(default)]
    pub complexity: Vec<ComplexitySpec>,
    #[serde(default)]
    pub exports: Vec<ExportSpec>,
    /// `include` entries as written (relative to the file).
    #[serde(default)]
    pub include: Vec<String>,
    /// The same includes, relative to the root config's directory (the keys of other sheets).
    #[serde(default)]
    pub include_files: Vec<String>,
}

/// A link the editor may draw from a brick to a sibling: one edge kind and scope, and what the
/// rules of that level decide for it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkOption {
    pub to: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    /// `prod` or `test`.
    pub scope: String,
    /// `allow` or `deny`.
    pub action: String,
    /// `file:line` of the deciding rule; `None` when `deps.default` decides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// Whether a rule `from -> to` (kinds, scope, action) would change what the level decides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCheck {
    /// Every edge it covers is already decided this way (or an equal rule exists).
    pub redundant: bool,
    /// Plain words for the UI ("already allowed by brickie.toml:12", "would allow 3 edges").
    pub why: String,
    /// `file:line` of the rule that already decides it (`None`: `deps.default`, or not
    /// redundant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}
