//! `publish`: the draft copied into the real files as one published version, once the code
//! matches its structure ([`Gate`]). No partial publish.

use super::super::commit::Stamp;
use super::super::tree::Files;
use super::super::{Design, Outcome, of_track, stale};
use super::Draft;
use crate::lock::Lock;
use anyhow::{Context, Result};
use brickie_model::codes::DESIGN_BLOCKED;
use brickie_model::design::{By, Gate, Track, Version};
use std::fmt;

/// `design/blocked`: the code does not match the draft's structure yet. Nothing was published.
#[derive(Debug)]
pub struct Blocked {
    /// What still blocks the publish.
    pub gate: Gate,
}

impl fmt::Display for Blocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{DESIGN_BLOCKED}]: the draft does not pass check yet ({} item(s)); \
             nothing was published. Run `brickie check` to see them.",
            self.gate.blocking.len()
        )?;
        for b in &self.gate.blocking {
            match b.message.starts_with(&b.code) {
                true => write!(f, "\n  {}", b.message)?,
                false => write!(f, "\n  {}: {}", b.code, b.message)?,
            }
        }
        Ok(())
    }
}

impl std::error::Error for Blocked {}

struct Ready {
    entries: Vec<Version>,
    published: Files,
    draft: Draft,
    v: u64,
}

impl Design {
    /// Under the lock: the draft exists and `base` is its head. The published head may have
    /// moved since the fork: the draft is the intent and overwrites it.
    fn ready(&self, lock: &Lock, base: u64) -> Result<Ready> {
        let (entries, published) = self.sync(lock)?;
        let draft = self
            .draft()?
            .context("there is no draft: every design edit starts one")?;
        stale::check(&of_track(&entries, Track::Draft), base)?;
        Ok(Ready {
            entries,
            published,
            draft,
            v: base,
        })
    }

    /// The publish checklist of the draft at `base` (nothing is written).
    pub fn publish_check(&self, base: u64) -> Result<Gate> {
        let lock = self.lock()?;
        let r = self.ready(&lock, base)?;
        self.gate(&r.draft.files)
    }

    /// Copy the draft at `base` into the real files as one published version by `by`, and
    /// remove the draft. Refused with [`Blocked`] while the gate has blocking items.
    pub fn publish(&self, base: u64, by: By, note: Option<String>) -> Result<Outcome> {
        let lock = self.lock()?;
        let r = self.ready(&lock, base)?;
        let gate = self.gate(&r.draft.files)?;
        if !gate.ok() {
            return Err(Blocked { gate }.into());
        }
        let over = overwritten(&r.entries, r.draft.forked_from);
        let note = note.unwrap_or_else(|| match over.as_deref() {
            Some(o) => format!("publish draft v{} (overwrote {o})", r.v),
            None => format!("publish draft v{}", r.v),
        });
        let stamp = Stamp {
            by,
            note: Some(note),
            ops: Vec::new(),
            fork: None,
            reverts: None,
        };
        let mut out = self.commit_published(&r.entries, &r.published, &r.draft.files, stamp)?;
        out.notes
            .extend(over.map(|o| format!("overwrote published {o}")));
        self.drop_draft()?;
        Ok(out)
    }
}

/// The published versions since the fork, as `v83..v101` (`v5` for one); `None` when none.
fn overwritten(entries: &[Version], forked_from: u64) -> Option<String> {
    let vs: Vec<u64> = (entries.iter())
        .filter(|e| e.track == Track::Published && e.v > forked_from)
        .map(|e| e.v)
        .collect();
    match (vs.first(), vs.last()) {
        (Some(a), Some(b)) if a == b => Some(format!("v{a}")),
        (Some(a), Some(b)) => Some(format!("v{a}..v{b}")),
        _ => None,
    }
}
