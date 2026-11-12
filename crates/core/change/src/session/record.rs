//! Recording ops and reviews (under the store lock).

use super::{Change, materialize, replay};
use crate::edit::{apply, canonical, key, target};
use crate::paths::join_rel;
use anyhow::{Context, Result, bail};
use brickie_model::change::{By, Entry, Op, OpRecord, ReviewEntry, State, Verdict};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The element key after the op (a rename moves it; a removed element is gone).
fn after_key(op: &Op, key: &str) -> Option<String> {
    match op {
        Op::RenameBrick { to, .. } => Some(format!("brick:{to}")),
        Op::RemoveBrick { .. } | Op::RemoveRule { .. } | Op::RemoveComplexity { .. } => None,
        _ => Some(key.to_string()),
    }
}

impl Change {
    /// Record an op on top of the current design (accepted + proposed ops). Human ops are
    /// accepted, agent ops proposed. An op that does not apply is an error and not recorded.
    pub fn record(&self, by: By, op: Op, note: Option<String>) -> Result<OpRecord> {
        self.snapshot_named(&op)?;
        let base = self.store.base()?;
        let root = self.root.clone();
        let entry = self.store.append(|entries, seq| {
            let mut ops = materialize(entries);
            let mut docs = replay(&base, Some(root), &mut ops, true)?;
            let file = op.file().to_string();
            let key = key(&docs, &op);
            let before = canonical(&docs, &file, &key);
            apply(&mut docs, &op).with_context(|| format!("{} on {file}", op.name()))?;
            let after_file = target(&docs, &op).unwrap_or_else(|| file.clone());
            let after = after_key(&op, &key).and_then(|k| canonical(&docs, &after_file, &k));
            Ok(Entry::Op(Box::new(OpRecord {
                seq,
                by,
                at: now(),
                op,
                file,
                key,
                before,
                after,
                note: note.filter(|n| !n.is_empty()),
                state: match by {
                    By::Human => State::Accepted,
                    By::Agent => State::Proposed,
                },
                review: None,
                conflict: None,
            })))
        })?;
        match entry {
            Entry::Op(o) => Ok(*o),
            Entry::Review(_) => unreachable!("record appends an op"),
        }
    }

    /// Accept or reject op `target`. Only a human reviews.
    pub fn review(
        &self,
        by: By,
        target: u64,
        verdict: Verdict,
        note: Option<String>,
    ) -> Result<ReviewEntry> {
        if by != By::Human {
            bail!("only a human reviews ops");
        }
        if verdict == Verdict::Retract {
            bail!("use retract to take back an op");
        }
        self.append_review(by, target, verdict, note)
    }

    /// Append a review entry of op `target` (any verdict; callers check who may give it).
    pub(super) fn append_review(
        &self,
        by: By,
        target: u64,
        verdict: Verdict,
        note: Option<String>,
    ) -> Result<ReviewEntry> {
        let entry = self.store.append(|entries, seq| {
            let ops = materialize(entries);
            let o = ops
                .iter()
                .find(|o| o.seq == target)
                .with_context(|| format!("no op with seq {target}"))?;
            let same = matches!(
                (o.state, verdict),
                (State::Accepted, Verdict::Accept)
                    | (State::Rejected, Verdict::Reject | Verdict::Retract)
            );
            if same {
                bail!("op {target} is already {:?}", o.state);
            }
            Ok(Entry::Review(ReviewEntry {
                seq,
                by,
                at: now(),
                target,
                verdict,
                note: note.filter(|n| !n.is_empty()),
            }))
        })?;
        match entry {
            Entry::Review(r) => Ok(r),
            Entry::Op(_) => unreachable!("review appends a review"),
        }
    }

    /// Add to `base/` the existing files an op names that the change did not snapshot (an
    /// existing preset included later).
    fn snapshot_named(&self, op: &Op) -> Result<()> {
        let mut named = vec![op.file().to_string()];
        if let Op::AddInclude { file, path } = op {
            named.push(join_rel(file, path));
        }
        let base = self.store.base()?;
        for rel in named {
            if base.contains_key(&rel) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(self.root.join(&rel)) {
                self.store.add_base(&rel, &bytes)?;
            }
        }
        Ok(())
    }
}
