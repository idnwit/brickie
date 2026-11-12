//! Taking ops back without rewriting the log: `retract` (the human's undo), `redo` (restore
//! the latest retracted op) and `reapply` (re-record a conflicting op's intent against the
//! current design).

use super::Change;
use super::replay::{materialize, replay};
use anyhow::{Context, Result, bail};
use brickie_model::change::{By, Entry, OpRecord, ReviewEntry, State, Verdict};

/// The redo stack (top last): ops the human retracted since their last new op, not restored.
pub fn redo_stack(entries: &[Entry]) -> Vec<u64> {
    let mut stack: Vec<u64> = Vec::new();
    for e in entries {
        match e {
            Entry::Op(o) if o.by == By::Human => stack.clear(),
            Entry::Review(r) if r.verdict == Verdict::Retract => stack.push(r.target),
            Entry::Review(r) if r.verdict == Verdict::Restore => stack.retain(|&t| t != r.target),
            // Another review of a retracted op (accepted or rejected by hand) ends its redo.
            Entry::Review(r) => stack.retain(|&t| t != r.target),
            Entry::Op(_) => {}
        }
    }
    stack
}

impl Change {
    /// Restore the most recently retracted human op (redo): a `restore` review marks it
    /// accepted again. Fails, recording nothing, when the redo stack is empty, the op does not
    /// apply to the current design any more, or restoring it would put a later op in conflict.
    pub fn redo(&self) -> Result<ReviewEntry> {
        let entries = self.store.entries()?;
        let seq = *redo_stack(&entries).last().context("nothing to redo")?;
        let base = self.store.base()?;
        let mut now = materialize(&entries);
        replay(&base, None, &mut now, false)?;
        let mut ops = now.clone();
        for o in ops.iter_mut().filter(|o| o.seq == seq) {
            o.state = State::Accepted;
        }
        replay(&base, None, &mut ops, false)?;
        // The op must apply, and must not break an op that applies now (replay is in seq
        // order, so a later op on the same element would turn into a conflict).
        let broken = ops.iter().zip(&now).find(|(o, n)| {
            o.conflict.is_some()
                && (o.seq == seq || n.conflict.is_none() && o.state == State::Accepted)
        });
        if let Some((o, _)) = broken {
            let why = o.conflict.clone().unwrap_or_default();
            bail!("cannot redo op {seq}: {why}");
        }
        self.append_review(By::Human, seq, Verdict::Restore, Some("redone".into()))
    }

    /// Take back the human op `seq` (`None`: the latest accepted human op). The op stays in
    /// the log, marked rejected by a `retract` review; later ops that depend on it become
    /// conflicts, as with a rejection.
    pub fn retract(&self, seq: Option<u64>) -> Result<ReviewEntry> {
        let log = self.log()?;
        let live = |o: &&OpRecord| o.by == By::Human && o.state == State::Accepted;
        let o = match seq {
            Some(s) => log
                .ops
                .iter()
                .find(|o| o.seq == s)
                .with_context(|| format!("no op with seq {s}"))?,
            None => log
                .ops
                .iter()
                .rev()
                .find(live)
                .context("no human op to retract")?,
        };
        if o.by != By::Human {
            bail!("op {} is an agent op: reject it instead", o.seq);
        }
        if !live(&o) {
            bail!("op {} is already rejected", o.seq);
        }
        self.append_review(By::Human, o.seq, Verdict::Retract, Some("retracted".into()))
    }

    /// Re-record the op `seq`, which is in conflict, as a human op against the current design,
    /// and reject the original (`re-applied as op N`). Returns the new op. Fails, recording
    /// nothing, when the op does not apply now either.
    pub fn reapply(&self, seq: u64) -> Result<OpRecord> {
        let log = self.log()?;
        let o = log
            .ops
            .iter()
            .find(|o| o.seq == seq)
            .with_context(|| format!("no op with seq {seq}"))?;
        if o.conflict.is_none() {
            bail!("op {seq} is not in conflict");
        }
        let note = format!("re-applied op {seq}");
        let rec = self.record(By::Human, o.op.clone(), Some(note))?;
        let why = format!("re-applied as op {}", rec.seq);
        self.append_review(By::Human, seq, Verdict::Reject, Some(why))?;
        Ok(rec)
    }
}
