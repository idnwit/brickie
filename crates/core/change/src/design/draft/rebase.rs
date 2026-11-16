//! `rebase`: the draft's ops replayed onto the new published head; `discard`: the draft
//! dropped. Both are versions of the draft track, so the history stays append-only.

use super::super::commit::Stamp;
use super::super::undo::segment_start;
use super::super::{Design, Outcome, history, of_track, stale, tree};
use crate::edit::{Docs, apply};
use anyhow::{Context, Result, anyhow, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::{By, Op, Track, Version};
use std::collections::BTreeMap;

impl Design {
    /// Replay the ops of the draft at `base` onto the published head, as one draft version
    /// based on it. A clash (an op whose target the published changes removed or took) fails
    /// with the op's own error; nothing is written. No silent merge of TOML.
    pub fn rebase(&self, base: u64, by: By) -> Result<Outcome> {
        let lock = self.lock()?;
        let (entries, published) = self.sync(&lock)?;
        let draft = self.draft()?.context("there is no draft to rebase")?;
        stale::check(&of_track(&entries, Track::Draft), base)?;
        let head = history::head(&entries)?.v;
        if head == draft.forked_from {
            let v = history::head_of(&entries, Track::Draft).context("the draft has no version")?;
            return Ok(Outcome {
                version: v.clone(),
                changed: false,
                scaffold: Vec::new(),
                notes: Vec::new(),
            });
        }
        let ops = live_ops(&entries)?;
        let mut docs = Docs::from_base(&published, Some(self.root.clone()))?;
        for op in &ops {
            apply(&mut docs, op).map_err(|e| {
                anyhow!(
                    "rebase onto v{head}: {} on {} clashes with the published changes: {e:#}; \
                     nothing was written. Undo that draft version or discard the draft",
                    op.name(),
                    op.file()
                )
            })?;
        }
        let new: tree::Files = (docs.texts().into_iter())
            .map(|(k, v)| (k, v.into_bytes()))
            .collect();
        self.preview(&new)?.load_config().map_err(|e| {
            anyhow!("{DESIGN_INVALID}: rebase onto v{head}: {e:#}; nothing was written")
        })?;
        let stamp = Stamp {
            by,
            note: Some(format!("rebase onto v{head}")),
            ops,
            fork: Some(head),
            reverts: None,
        };
        self.commit_draft(&entries, &draft.files, &new, stamp)
    }

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

/// The ops of the current draft that still apply, in order: an undo turns the version it
/// reverts off (an undo of an undo turns it on again).
fn live_ops(entries: &[Version]) -> Result<Vec<Op>> {
    let start = segment_start(entries);
    let draft: Vec<&Version> = (entries.iter())
        .filter(|e| e.track == Track::Draft && e.v >= start)
        .collect();
    if draft.is_empty() {
        bail!("the draft's first version is no longer kept; discard the draft and redo it");
    }
    let mut on: BTreeMap<u64, bool> = BTreeMap::new();
    for e in &draft {
        match e.reverts {
            Some(t) => {
                let o = origin(&draft, t);
                let flag = on.entry(o).or_insert(true);
                *flag = !*flag;
            }
            None => {
                on.insert(e.v, true);
            }
        }
    }
    Ok((draft.iter())
        .filter(|e| e.reverts.is_none() && on.get(&e.v) == Some(&true))
        .flat_map(|e| e.ops.clone())
        .collect())
}

/// The version an undo chain ends at (the undo of an undo of `v` is about `v`).
fn origin(draft: &[&Version], mut t: u64) -> u64 {
    while let Some(next) = (draft.iter()).find(|e| e.v == t).and_then(|e| e.reverts) {
        t = next;
    }
    t
}
