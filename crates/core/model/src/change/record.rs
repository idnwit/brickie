//! The change log: op records and review entries, each with its own `seq`.

use super::Op;
use serde::{Deserialize, Serialize};

/// Who wrote an entry. Human ops are accepted when recorded; agent ops wait for review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum By {
    Human,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Accept,
    Reject,
    /// The human takes back their own op (undo): the op counts as rejected, the log keeps it.
    Retract,
    /// The human restores an op they retracted (redo): the op counts as accepted again.
    Restore,
}

/// The review that decided an op: its own `seq`, the reviewer and the note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub seq: u64,
    pub by: By,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A logged op. `key` names the element within `file` (`brick:<name>`, `rule:<i>`,
/// `complexity:<i>`, `exports`, `deps`, `include:<path>`, `file`); `before` / `after` are its
/// canonical TOML (`None`: absent). `state` and `review` are derived from the review entries;
/// `conflict` is set when the op no longer replays at its `seq` (its `before` does not match).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpRecord {
    pub seq: u64,
    pub by: By,
    /// Unix seconds.
    pub at: u64,
    pub op: Op,
    pub file: String,
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Review>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<String>,
}

/// A review of op `target`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEntry {
    pub seq: u64,
    pub by: By,
    pub at: u64,
    pub target: u64,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One line of `ops.json` (append-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry", rename_all = "lowercase")]
pub enum Entry {
    Op(Box<OpRecord>),
    Review(ReviewEntry),
}

impl Entry {
    pub fn seq(&self) -> u64 {
        match self {
            Entry::Op(o) => o.seq,
            Entry::Review(r) => r.seq,
        }
    }
}
