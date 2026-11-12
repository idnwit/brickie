//! One config file of the preview as the editor's tables show it: its own `[deps]` settings,
//! rules, complexity rules, exports and includes (not merged with what it includes), so every
//! row maps to the `file` and `index` of an op.

use super::{ComplexitySpec, ExportSpec, RuleSpec};
use serde::{Deserialize, Serialize};

/// The editable content of one physical config file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSheet {
    /// `deps.default` / `deps.cycles` as written (`None`: not set in this file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycles: Option<String>,
    /// `[[deps.rule]]` in file order (the `index` of `set_rule` / `remove_rule`).
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
    /// The same includes, workspace-relative (the keys of other sheets).
    #[serde(default)]
    pub include_files: Vec<String>,
}
