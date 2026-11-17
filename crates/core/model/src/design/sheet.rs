//! What the viewer's editor reads of the design besides the tree: one config file's own rows
//! ([`FileSheet`]), the links a brick may draw ([`LinkOption`]) and what a new link would
//! be ([`LinkCheck`]).

use super::{ComplexitySpec, RuleSpec};
use serde::{Deserialize, Serialize};

/// The editable content of one physical config file, not merged with what it includes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSheet {
    /// `[[rule]]` in file order.
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    /// `[[link]]` in file order.
    #[serde(default)]
    pub links: Vec<RuleSpec>,
    /// `[[complexity]]` in file order.
    #[serde(default)]
    pub complexity: Vec<ComplexitySpec>,
    #[serde(default)]
    pub exports: Vec<String>,
    /// `include` entries as written (relative to the file).
    #[serde(default)]
    pub include: Vec<String>,
    /// The same includes, relative to the root config's directory (the keys of other sheets).
    #[serde(default)]
    pub include_files: Vec<String>,
}

/// A link the editor may draw from a brick to a sibling: one edge kind and scope, and whether a
/// rule of that level allows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkOption {
    pub to: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    /// `prod` or `test`.
    pub scope: String,
    /// Some rule allows it (a link into an independent brick needs none).
    pub allowed: bool,
    /// `file:line` of the allowing rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// What a new link `from -> to` (kinds, scope) would be: a duplicate, allowed by the rules,
/// or denied (`design/link-denied`: it may be drafted but blocks publishing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCheck {
    /// An identical link exists already.
    pub redundant: bool,
    /// Every kind and scope it names is allowed by a rule.
    pub allowed: bool,
    /// Plain words for the UI ("allowed by brickie.toml:12", "no rule allows ...").
    pub why: String,
    /// `file:line` of the identical link, or of a rule that allows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}
