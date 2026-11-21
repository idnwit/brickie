//! `discard`: the draft dropped. It is a version of the draft track, so the history stays
//! append-only.

use super::super::{Design, Outcome, history, of_track, stale, tree};
use anyhow::{Context, Result};
use brickie_model::design::{By, Track, Version};

impl Design {
    /// Drop the draft at `base`. The history keeps a `discard draft` version.
    pub fn discard(&self, base: u64, by: By) -> Result<Outcome> {
        let lock = self.lock()?;
        let (entries, _) = self.sync(&lock)?;
        let draft = self.draft()?.context("there is no draft to discard")?;
        stale::check(&of_track(&entries, Track::Draft), base)?;
        let head = history::head_of(&entries, Track::Draft).context("the draft has no version")?;
        let empty = tree::Files::new();
        let version = Version {
            v: history::next(&entries),
            track: Track::Draft,
            forked_from: None,
            reverts: None,
            hash: tree::hash(&empty),
            prev: Some(head.hash.clone()),
            by,
            at: stale::now(),
            note: Some(format!(
                "discard draft (forked from v{})",
                draft.forked_from
            )),
            ops: Vec::new(),
            diff: Vec::new(),
            files: Vec::new(),
            removed: Vec::new(),
            full: true,
        };
        self.drop_draft()?;
        self.history.append(&version, &empty)?;
        Ok(Outcome {
            version,
            changed: true,
            scaffold: Vec::new(),
            notes: Vec::new(),
        })
    }
}
