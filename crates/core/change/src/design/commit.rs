//! Recording a write: the draft stored (or the published files written) and the version
//! appended to the history.

use super::tree::{self, Files};
use super::{Design, history, stale};
use crate::lock::write_atomic;
use crate::report::diff::diff;
use anyhow::{Context, Result};
use brickie_model::design::{By, Op, Track, Version};

/// Who wrote a version, why, and how it relates to the versions before.
pub(super) struct Stamp {
    pub by: By,
    pub note: Option<String>,
    pub ops: Vec<Op>,
    /// The write starts a draft based on this published version (a fork or a rebase).
    pub fork: Option<u64>,
    /// An undo: the version it reverts.
    pub reverts: Option<u64>,
}

fn changes(old: &Files, new: &Files) -> (Vec<String>, Vec<String>) {
    let changed = (new.iter())
        .filter(|(p, b)| old.get(*p) != Some(*b))
        .map(|(p, _)| p.clone())
        .collect();
    let gone = (old.keys())
        .filter(|p| !new.contains_key(*p))
        .cloned()
        .collect();
    (changed, gone)
}

fn unchanged(head: &Version) -> super::Outcome {
    super::Outcome {
        version: head.clone(),
        changed: false,
        scaffold: Vec::new(),
        notes: Vec::new(),
    }
}

impl Design {
    /// Store `new` as the draft and append the draft version. A fork that changes nothing
    /// creates no draft.
    pub(super) fn commit_draft(
        &self,
        entries: &[Version],
        old: &Files,
        new: &Files,
        s: Stamp,
    ) -> Result<super::Outcome> {
        let draft = self.draft()?;
        let head = match &draft {
            Some(_) => {
                history::head_of(entries, Track::Draft).context("the draft has no version")?
            }
            None => history::head(entries)?,
        };
        let (changed, gone) = changes(old, new);
        if changed.is_empty() && gone.is_empty() && s.fork.is_none_or(|_| draft.is_none()) {
            return Ok(unchanged(head));
        }
        let forked_from = match (s.fork, &draft) {
            (Some(f), _) => f,
            (None, Some(d)) => d.forked_from,
            (None, None) => anyhow::bail!("no draft to write"),
        };
        self.store_draft(forked_from, new)?;
        let full = s.fork.is_some();
        let version = Version {
            v: history::next(entries),
            track: Track::Draft,
            forked_from: s.fork,
            reverts: s.reverts,
            hash: tree::hash(new),
            prev: Some(head.hash.clone()),
            by: s.by,
            at: stale::now(),
            note: s.note.filter(|n| !n.trim().is_empty()),
            ops: s.ops,
            diff: diff(old, new),
            files: match full {
                true => new.keys().cloned().collect(),
                false => changed,
            },
            removed: gone,
            full,
        };
        self.history.append(&version, new)?;
        Ok(super::Outcome {
            version,
            changed: true,
            scaffold: Vec::new(),
            notes: Vec::new(),
        })
    }

    /// Write the published files that differ from `old`, append the published version. An undo
    /// deletes the files it takes away; a publish leaves files only dropped from the tree
    /// (ungroup) on disk.
    pub(super) fn commit_published(
        &self,
        entries: &[Version],
        old: &Files,
        new: &Files,
        s: Stamp,
    ) -> Result<super::Outcome> {
        let head = history::head(entries)?;
        let (changed, gone) = changes(old, new);
        if changed.is_empty() && gone.is_empty() {
            return Ok(unchanged(head));
        }
        for p in &changed {
            write_atomic(&self.root.join(p), &new[p])?;
        }
        if s.reverts.is_some() {
            for p in gone.iter() {
                let _ = std::fs::remove_file(self.root.join(p));
            }
        }
        let disk = self.published_files()?;
        let version = Version {
            v: history::next(entries),
            track: Track::Published,
            forked_from: None,
            reverts: s.reverts,
            hash: tree::hash(&disk),
            prev: Some(head.hash.clone()),
            by: s.by,
            at: stale::now(),
            note: s.note.filter(|n| !n.trim().is_empty()),
            ops: s.ops,
            diff: diff(old, &disk),
            files: changed
                .into_iter()
                .filter(|p| disk.contains_key(p))
                .collect(),
            removed: (old.keys())
                .filter(|p| !disk.contains_key(*p))
                .cloned()
                .collect(),
            full: false,
        };
        self.history.append(&version, &disk)?;
        Ok(super::Outcome {
            version,
            changed: true,
            scaffold: Vec::new(),
            notes: Vec::new(),
        })
    }
}
