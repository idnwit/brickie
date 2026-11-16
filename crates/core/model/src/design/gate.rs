//! The publish gate's result: what the code must still do before the draft can be published.

use serde::{Deserialize, Serialize};

/// One thing the code must do before the draft can be published.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    pub code: String,
    pub message: String,
    /// The draft config file the item is about (workspace-relative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The brick concerned (`root/web/api`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brick: Option<String>,
    /// The member selector concerned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
    /// The module concerned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

/// The publish checklist of the draft.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    /// Structure items: the publish is refused while any is left.
    pub blocking: Vec<Blocker>,
    /// Dependency and rule findings of the draft (`code: message`); they never block.
    pub info: Vec<String>,
}

impl Gate {
    pub fn ok(&self) -> bool {
        self.blocking.is_empty()
    }
}
