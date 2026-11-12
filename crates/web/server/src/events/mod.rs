//! Server-Sent Events (`GET /api/events`): one watcher per server notices what changed on disk
//! (the change log, `cache.json`, the `brickie*.toml` tree) or in the measure job, whoever wrote
//! it, and fans it out to every page over a `tokio::sync::broadcast` channel.
//!
//! Decision: a cheap mtime watcher (a stat of a few files every 200 ms, the toml tree every 2 s)
//! instead of the `notify` crate: no new dependency, no inotify limits, the same behaviour on
//! every platform and filesystem (network mounts, containers), and atomic renames (how
//! `cache.json` and `ops.json` are written) are seen as a new mtime.

pub(crate) mod scan;

pub(crate) use scan::{Snapshot, snapshot};

use crate::AppState;
use brickie_web_api::ServerEvent;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::broadcast;

/// How often the watcher looks.
const TICK: Duration = Duration::from_millis(200);
/// Every how many ticks it also walks the toml tree.
const TREE_EVERY: u32 = 10;

/// The watcher's last view and the channel its events go out on.
pub(crate) struct Hub {
    tx: broadcast::Sender<ServerEvent>,
    last: Mutex<Option<Snapshot>>,
}

impl Hub {
    pub(crate) fn new() -> Hub {
        Hub {
            tx: broadcast::channel(64).0,
            last: Mutex::new(None),
        }
    }

    /// The last snapshot (`None` before the first look).
    pub(crate) fn last(&self) -> Option<Snapshot> {
        self.last.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }

    /// Compare `now` with the last view, remember it and send what differs (nothing the first
    /// time). Returns the events sent.
    pub(crate) fn update(&self, now: Snapshot) -> Vec<ServerEvent> {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        let events = match last.as_ref() {
            Some(before) => before.diff(&now),
            None => Vec::new(),
        };
        *last = Some(now);
        for e in &events {
            // No receiver (no page open) is fine.
            let _ = self.tx.send(e.clone());
        }
        events
    }
}

/// Run the watcher for the server's lifetime.
pub(crate) fn spawn(st: AppState) {
    tokio::spawn(async move {
        let mut n: u32 = 0;
        let mut tree = 0;
        loop {
            if n.is_multiple_of(TREE_EVERY) {
                let s = st.clone();
                tree = tokio::task::spawn_blocking(move || scan::tree_stamp(&s.root()))
                    .await
                    .unwrap_or(tree);
            }
            let s = st.clone();
            if let Ok(now) = tokio::task::spawn_blocking(move || snapshot(&s, tree)).await {
                st.events.update(now);
            }
            n = n.wrapping_add(1);
            tokio::time::sleep(TICK).await;
        }
    });
}
