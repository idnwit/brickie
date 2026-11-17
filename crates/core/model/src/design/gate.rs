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

/// The publish gate of the draft: `check` run on it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    /// What `check` reports on the draft (errors, planned bricks, cache): the publish is
    /// refused while any is left.
    pub blocking: Vec<Blocker>,
}

impl Gate {
    pub fn ok(&self) -> bool {
        self.blocking.is_empty()
    }
}
