//! One version of the design: who made it, what changed, and how to find its files again.

use super::Op;
use serde::{Deserialize, Serialize};

/// Who made a version. `External` is a hand edit of the files found on the next read or write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum By {
    Human,
    Agent,
    External,
}

impl By {
    /// The lowercase name, as in JSON.
    pub fn name(self) -> &'static str {
        match self {
            By::Human => "human",
            By::Agent => "agent",
            By::External => "external",
        }
    }
}

/// Which tree a version belongs to. `Published` is the real `brickie.toml` tree that `check`
/// reads; `Draft` is the one draft under `.brickie/draft/` that every design edit writes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Track {
    #[default]
    Published,
    Draft,
}

impl Track {
    /// The lowercase name, as in JSON.
    pub fn name(self) -> &'static str {
        match self {
            Track::Published => "published",
            Track::Draft => "draft",
        }
    }
}

/// One line of `.brickie/history/log.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    /// The version number, counting up from 1 for the whole design.
    pub v: u64,
    /// The tree this version belongs to; version numbers are shared by both tracks.
    #[serde(default)]
    pub track: Track,
    /// A draft version that starts a draft (fork or rebase): the published version it is
    /// based on. Its snapshot is `full`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<u64>,
    /// An undo: the version whose file changes it reverts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverts: Option<u64>,
    /// Hash of every config file of the tree (sorted path and bytes), hex.
    pub hash: String,
    /// The hash of the version before (`None` for the first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    /// Who made the version.
    pub by: By,
    /// Unix seconds.
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// A free-text note given with the write.
    pub note: Option<String>,
    /// The ops of the write (empty for an external edit or an undo).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<Op>,
    /// The semantic diff from the version before, one line per element.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diff: Vec<String>,
    /// Config files this version wrote (their bytes are in its snapshot).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// Config files that left the tree in this version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// The snapshot holds every file of the tree (the oldest kept version).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub full: bool,
}

impl Version {
    /// `v15`.
    pub fn name(&self) -> String {
        format!("v{}", self.v)
    }

    /// The first 8 hex digits of the hash.
    pub fn short(&self) -> &str {
        &self.hash[..self.hash.len().min(8)]
    }
}
