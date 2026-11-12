//! What the watcher looks at: a [`Snapshot`] of the change log, the cache, the measure job and
//! the config tree, and the events between two snapshots.

use crate::AppState;
use brickie_change::Change;
use brickie_web_api::{MeasureJob, ServerEvent};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// The state the events report, with the file stamps they were read from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    /// `(open, head)` of the change and the stamp of its log.
    pub(crate) change: (bool, u64),
    change_stamp: (u64, u64),
    /// `(written at in seconds, stamp of time and size)` of `cache.json`.
    pub(crate) cache: (u64, u64),
    pub(crate) measure: MeasureJob,
    pub(crate) config: u64,
}

/// `(mtime in ns, size)` of a file, `(0, 0)` when missing.
fn stamp(p: &Path) -> (u64, u64) {
    std::fs::metadata(p).map_or((0, 0), |m| {
        let t = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as u64);
        (t, m.len())
    })
}

/// A stamp of every `brickie*.toml` under `root` (hidden folders and `target` skipped).
pub(crate) fn tree_stamp(root: &Path) -> u64 {
    let mut acc: u64 = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let p = e.path();
            if p.is_dir() {
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    stack.push(p);
                }
            } else if name.starts_with("brickie") && name.ends_with(".toml") {
                let (t, len) = stamp(&p);
                // Order-free mix of each file's path, time and size.
                let h = name.len() as u64 ^ t.rotate_left(17) ^ len.rotate_left(41);
                acc = acc.wrapping_add(h.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            }
        }
    }
    acc
}

/// Read the current state (`tree`: the last [`tree_stamp`]); the change log is parsed only
/// when its file changed since `prev`.
pub(crate) fn snapshot(st: &AppState, tree: u64) -> Snapshot {
    let config = st.config();
    let ops = st.root().join(".brickie/change/ops.json");
    let mut change_stamp = stamp(&ops);
    let prev = st.events.last();
    let change = match prev {
        Some(p) if p.change_stamp == change_stamp => p.change,
        _ => match Change::load(&config).and_then(|c| c.status()) {
            Ok(s) => (true, s.head),
            Err(_) => {
                // Not readable (none open, or `Change::open` is between writing `ops.json`
                // and `intent`): never remember this stamp, so the next look reads again.
                change_stamp = (u64::MAX, u64::MAX);
                (Change::is_open(&config), 0)
            }
        },
    };
    let (t, len) = stamp(&st.cache_file());
    Snapshot {
        change,
        change_stamp,
        cache: (t / 1_000_000_000, len ^ t),
        measure: st.measure.status(),
        config: tree,
    }
}

impl Snapshot {
    /// One event per part that differs in `now`.
    pub(crate) fn diff(&self, now: &Snapshot) -> Vec<ServerEvent> {
        let mut out = Vec::new();
        if self.change != now.change {
            out.push(now.change_event());
        }
        if self.cache != now.cache {
            out.push(now.cache_event());
        }
        if self.measure != now.measure {
            out.push(ServerEvent::Measure {
                job: now.measure.clone(),
            });
        }
        if self.config != now.config {
            out.push(ServerEvent::Config { stamp: now.config });
        }
        out
    }

    /// The current state as one event of each kind (sent first on every connection).
    pub(crate) fn events(&self) -> Vec<ServerEvent> {
        vec![
            self.change_event(),
            self.cache_event(),
            ServerEvent::Measure {
                job: self.measure.clone(),
            },
            ServerEvent::Config { stamp: self.config },
        ]
    }

    fn change_event(&self) -> ServerEvent {
        ServerEvent::Change {
            open: self.change.0,
            head: self.change.1,
        }
    }

    fn cache_event(&self) -> ServerEvent {
        ServerEvent::Cache {
            measured_at: self.cache.0,
            stamp: self.cache.1,
        }
    }
}
