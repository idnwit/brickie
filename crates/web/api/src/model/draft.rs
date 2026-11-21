//! The request bodies of the draft's lifecycle: publish and discard. Each is based on
//! the draft's head version; the reply is an [`EditReply`](crate::EditReply).

use crate::Based;
use serde::{Deserialize, Serialize};

/// Publish the draft at `base`. `dry_run`: run the gate and write nothing
/// (refused like a real publish while it blocks).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishRequest {
    pub base: u64,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Discard the draft at `base`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftRequest {
    pub base: u64,
}

impl Based for PublishRequest {
    fn rebase(&mut self, v: u64) {
        self.base = v;
    }
}

impl Based for DraftRequest {
    fn rebase(&mut self, v: u64) {
        self.base = v;
    }
}
