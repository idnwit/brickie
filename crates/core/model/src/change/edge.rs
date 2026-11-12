//! Rule edges of the design: the ones a change adds that no measured edge backs yet, and the
//! answer to "would this link change anything?".

use super::{By, State};
use serde::{Deserialize, Serialize};

/// A `[[deps.rule]]` the change adds (not in the design as opened) that no measured module
/// edge backs at its level: the canvas draws it dashed, `--todo` lists the allow ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedEdge {
    /// Level path the rule applies to (`root`, `root/web`).
    pub level: String,
    /// Physical file of the rule and its index among that file's rules (the `index` of
    /// `set_rule` / `remove_rule`).
    pub file: String,
    pub rule: usize,
    /// Selectors as written: a brick name, `tag:<t>` or `*`.
    pub from: String,
    pub to: String,
    /// Empty: all kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    /// `prod`, `test` or `all`.
    pub scope: String,
    /// `allow` or `deny`.
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// The last op that wrote the rule, its author and review state (`None` when no single op
    /// wrote this exact rule, e.g. lifted by a move).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<By>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,
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
