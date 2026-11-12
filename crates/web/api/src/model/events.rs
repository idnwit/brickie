//! Server-Sent Events of `GET /api/events`: what changed on disk or in the measure job, from any
//! writer (the page, the CLI, a coding agent). The SSE `event:` name is the variant's `kind`;
//! `data:` is the JSON of the whole event. On (re)connect the server first sends one event of
//! each kind with the current state, so a client resyncs without replaying history.

use super::MeasureJob;
use serde::{Deserialize, Serialize};

/// `GET` path of the event stream (`text/event-stream`).
pub const EVENTS_PATH: &str = "/api/events";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ServerEvent {
    /// The open change's log moved (`head`: its last seq), or a change was opened, published
    /// or discarded (`open`).
    Change { open: bool, head: u64 },
    /// `cache.json` was replaced: when (Unix seconds), and a stamp of its exact time and size.
    Cache { measured_at: u64, stamp: u64 },
    /// The server's measure job moved on.
    Measure { job: MeasureJob },
    /// A `brickie*.toml` of the tree changed (a stamp of the files' times and sizes).
    Config { stamp: u64 },
}

impl ServerEvent {
    /// The SSE `event:` name.
    pub fn kind(&self) -> &'static str {
        match self {
            ServerEvent::Change { .. } => "change",
            ServerEvent::Cache { .. } => "cache",
            ServerEvent::Measure { .. } => "measure",
            ServerEvent::Config { .. } => "config",
        }
    }
}
