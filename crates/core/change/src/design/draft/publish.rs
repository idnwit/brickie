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
    pub gate: Gate,
}

impl fmt::Display for Blocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{DESIGN_BLOCKED}]: the code does not match the draft yet ({} item(s)); \
             nothing was published. See `brickie design check`.",
            self.gate.blocking.len()
        )?;
        for b in &self.gate.blocking {
            write!(f, "\n  {}", b.message)?;
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
    /// Under the lock: the draft exists, `base` is its head and the published head is still
    /// the version it was forked from.
    fn ready(&self, lock: &Lock, base: u64) -> Result<Ready> {
        let (entries, published) = self.sync(lock)?;
        let draft = self
            .draft()?
            .context("there is no draft: every design edit starts one")?;
        stale::check(&of_track(&entries, Track::Draft), base)?;
        let pubs = of_track(&entries, Track::Published);
        stale::published(&pubs, draft.forked_from, base)?;
        Ok(Ready {
            entries,
            published,
            draft,
            v: base,
        })
    }

    /// The gate of the working design (the draft, else the published files) against the
    /// code: exactly what `publish` would check. Nothing is written.
    pub fn check_working(&self) -> Result<Gate> {
        self.gate(&self.files()?)
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
        let stamp = Stamp {
            by,
            note: Some(note.unwrap_or_else(|| format!("publish draft v{}", r.v))),
            ops: Vec::new(),
            fork: None,
            reverts: None,
        };
        let out = self.commit_published(&r.entries, &r.published, &r.draft.files, stamp)?;
        self.drop_draft()?;
        Ok(out)
    }
}
