//! `undo`: a version's file changes reverted as a new version of its own track. A draft
//! version is undone in the draft; a published version (a publish, a hand edit) by restoring
//! the published bytes.

use super::commit::Stamp;
use super::{Design, Outcome, of_track, stale};
use anyhow::{Context, Result, bail};
use brickie_model::codes::DESIGN_INVALID;
use brickie_model::design::{By, Track, Version};
use std::collections::BTreeSet;

impl Design {
    /// Revert the file changes of version `target` (default: the working head) as a new version
    /// of its track; `base` is that track's head. Refused when a file it changed is no longer
    /// as it left it (a later version changed it and was not reverted), and for an agent when
    /// `target` was made by a human or by hand.
    pub fn undo(&self, base: u64, by: By, target: Option<u64>) -> Result<Outcome> {
        let lock = self.lock()?;
        let (entries, published) = self.sync(&lock)?;
        let draft = self.draft()?;
        let n = match target {
            Some(n) => n,
            None => self.work_head(&entries)?.v,
        };
        let t = (entries.iter())
            .find(|e| e.v == n)
            .with_context(|| format!("v{n} is not in the history"))?;
        let old = match (t.track, draft) {
            (Track::Published, _) => published,
            (Track::Draft, Some(d)) if n >= segment_start(&entries) => d.files,
            (Track::Draft, Some(_)) => bail!(
                "v{n} is before the draft's fork (v{}); undo that version instead",
                segment_start(&entries)
            ),
            (Track::Draft, None) => {
                bail!("v{n} belongs to a draft that is gone (published or discarded)")
            }
        };
        allowed(t, by)?;
        stale::check(&of_track(&entries, t.track), base)?;
        let touched: BTreeSet<&String> = t.files.iter().chain(&t.removed).collect();
        let after = self.history.state(&entries, n)?;
        // A later version that changed a file and was reverted since leaves it as `n` did.
        if let Some(f) = touched.iter().find(|f| old.get(**f) != after.get(**f)) {
            let later = (entries.iter().rev()).find(|e| {
                e.v > n && e.track == t.track && e.files.iter().chain(&e.removed).any(|x| x == *f)
            });
            let v = later.map_or(n, |e| e.v);
            bail!("v{v} changed {f} after v{n}; undo v{v} first");
        }
        let before = match t.forked_from {
            Some(f) => self.history.state(&entries, f).with_context(|| {
                format!("v{n} starts the draft from v{f}, which is no longer kept")
            })?,
            None => self.history.state_before(&entries, n)?,
        };
        let mut new = old.clone();
        for f in touched {
            match before.get(f) {
                Some(b) => new.insert(f.clone(), b.clone()),
                None => new.remove(f),
            };
        }
        self.preview(&new)?
            .load_config()
            .map_err(|e| anyhow::anyhow!("{DESIGN_INVALID}: {e:#}; nothing was written"))?;
        let stamp = Stamp {
            by,
            note: Some(format!("undo v{n}")),
            ops: Vec::new(),
            fork: None,
            reverts: Some(n),
        };
        match t.track {
            Track::Draft => self.commit_draft(&entries, &old, &new, stamp),
            Track::Published => self.commit_published(&entries, &old, &new, stamp),
        }
    }
}

/// An agent may undo only its own versions, and restore files a hand edit deleted.
fn allowed(t: &Version, by: By) -> Result<()> {
    if t.full && t.forked_from.is_none() {
        bail!(
            "v{} is the oldest kept version; there is nothing before it to go back to",
            t.v
        );
    }
    let restores = t.by == By::External && t.files.is_empty() && !t.removed.is_empty();
    if by == By::Agent && t.by != By::Agent && !restores {
        bail!(
            "v{} was made by {}; an agent never undoes it (ask the human)",
            t.v,
            t.by.name()
        );
    }
    Ok(())
}

/// The first version of the current draft: its last fork.
pub(super) fn segment_start(entries: &[Version]) -> u64 {
    (entries.iter().rev())
        .find(|e| e.track == Track::Draft && e.forked_from.is_some())
        .map_or(0, |e| e.v)
}
