//! Taking ops back without rewriting the log: `retract` (the human's undo) and `reapply`
//! (re-record a conflicting op's intent against the current design).

use super::Change;
use anyhow::{Context, Result, bail};
use brickie_model::change::{By, OpRecord, ReviewEntry, State, Verdict};

impl Change {
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
