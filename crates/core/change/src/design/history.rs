//! `.brickie/history/` (git-ignored): `log.jsonl`, one [`Version`] per line, and `v<N>/`, the
//! bytes of the config files version N wrote (every file for a `full` version), by
//! workspace-relative path with `..` stored as `^`. At most [`CAP`] versions are kept; the
//! oldest kept one of each track is then rewritten as a full snapshot. The state of a version
//! is replayed from the versions of its own track only.

use super::tree::Files;
use crate::lock::write_atomic;
use anyhow::{Context, Result, bail};
use brickie_model::design::{Track, Version};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CAP: usize = 200;

pub struct History {
    dir: PathBuf,
}

pub(super) fn encode(rel: &str) -> String {
    let parts: Vec<&str> = rel
        .split('/')
        .map(|c| if c == ".." { "^" } else { c })
        .collect();
    parts.join("/")
}

impl History {
    pub fn at(root: &Path) -> History {
        History {
            dir: root.join(".brickie/history"),
        }
    }

    fn log(&self) -> PathBuf {
        self.dir.join("log.jsonl")
    }

    fn snap(&self, v: u64) -> PathBuf {
        self.dir.join(format!("v{v}"))
    }

    pub fn entries(&self) -> Result<Vec<Version>> {
        let Ok(text) = std::fs::read_to_string(self.log()) else {
            return Ok(Vec::new());
        };
        let lines = text.lines().filter(|l| !l.trim().is_empty());
        lines
            .map(|l| serde_json::from_str(l).context("parse .brickie/history/log.jsonl"))
            .collect()
    }

    /// Store the snapshot of `v` (its `files`, read from `tree`), then append its line.
    pub fn append(&self, v: &Version, tree: &Files) -> Result<()> {
        for rel in &v.files {
            let bytes = tree
                .get(rel)
                .with_context(|| format!("{rel} is not in the tree"))?;
            write_atomic(&self.snap(v.v).join(encode(rel)), bytes)?;
        }
        std::fs::create_dir_all(&self.dir)?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log())
            .context("open .brickie/history/log.jsonl")?;
        writeln!(f, "{}", serde_json::to_string(v)?)?;
        drop(f);
        self.compact()
    }

    /// The config files as they were at version `v` (in its own track).
    pub fn state(&self, entries: &[Version], v: u64) -> Result<Files> {
        let track = (entries.iter().find(|e| e.v == v))
            .with_context(|| format!("v{v} is not in the history"))?
            .track;
        let mine: Vec<&Version> = entries.iter().filter(|e| e.track == track).collect();
        let start = mine
            .iter()
            .rposition(|e| e.full && e.v <= v)
            .with_context(|| format!("v{v} is not in the history"))?;
        let mut out = Files::new();
        for e in mine[start..].iter().take_while(|e| e.v <= v) {
            if e.full {
                out.clear();
            }
            for rel in &e.files {
                let p = self.snap(e.v).join(encode(rel));
                let bytes = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
                out.insert(rel.clone(), bytes);
            }
            for rel in &e.removed {
                out.remove(rel);
            }
        }
        Ok(out)
    }

    /// The state before version `v`: the previous version of its track.
    pub fn state_before(&self, entries: &[Version], v: u64) -> Result<Files> {
        let track = (entries.iter().find(|e| e.v == v))
            .with_context(|| format!("v{v} is not in the history"))?
            .track;
        let prev = (entries.iter().rev())
            .find(|e| e.track == track && e.v < v)
            .with_context(|| format!("no version before v{v} is kept"))?;
        self.state(entries, prev.v)
    }

    /// Keep the last [`CAP`] versions and the head of each track: the first kept one of each
    /// track becomes full.
    fn compact(&self) -> Result<()> {
        let mut entries = self.entries()?;
        if entries.len() <= CAP {
            return Ok(());
        }
        let drop_n = entries.len() - CAP;
        let mut keep: Vec<bool> = (0..entries.len()).map(|i| i >= drop_n).collect();
        for track in [Track::Published, Track::Draft] {
            if let Some(h) = entries.iter().rposition(|e| e.track == track) {
                keep[h] = true;
            }
        }
        for track in [Track::Published, Track::Draft] {
            let Some(i) = (0..entries.len()).find(|i| keep[*i] && entries[*i].track == track)
            else {
                continue;
            };
            if !entries[i].full {
                self.make_full(&mut entries, i)?;
            }
        }
        for (e, _) in entries.iter().zip(&keep).filter(|(_, k)| !**k) {
            let _ = std::fs::remove_dir_all(self.snap(e.v));
        }
        let mut kept = keep.iter();
        entries.retain(|_| *kept.next().unwrap_or(&true));
        let mut text = String::new();
        for e in &entries {
            text.push_str(&serde_json::to_string(e)?);
            text.push('\n');
        }
        write_atomic(&self.log(), text.as_bytes())
    }

    /// Rewrite the snapshot of `entries[i]` as the whole state of its track.
    fn make_full(&self, entries: &mut [Version], i: usize) -> Result<()> {
        let first = entries[i].v;
        let full = self.state(entries, first)?;
        let tmp = self.dir.join("full.tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        for (rel, bytes) in &full {
            write_atomic(&tmp.join(encode(rel)), bytes)?;
        }
        let _ = std::fs::remove_dir_all(self.snap(first));
        if full.is_empty() {
            std::fs::create_dir_all(&tmp)?;
        }
        std::fs::rename(&tmp, self.snap(first)).context("store the full snapshot")?;
        let e = &mut entries[i];
        e.full = true;
        e.files = full.keys().cloned().collect();
        e.removed.clear();
        Ok(())
    }
}

/// The last version of `track`.
pub fn head_of(entries: &[Version], track: Track) -> Option<&Version> {
    entries.iter().rev().find(|e| e.track == track)
}

/// The last published version.
pub fn head(entries: &[Version]) -> Result<&Version> {
    match head_of(entries, Track::Published) {
        Some(h) => Ok(h),
        None => bail!("the design has no history yet"),
    }
}

/// The number the next version gets (shared by both tracks).
pub fn next(entries: &[Version]) -> u64 {
    entries.last().map_or(1, |e| e.v + 1)
}
