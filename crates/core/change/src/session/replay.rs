//! The log materialized: op records with their review state, replayed onto the base in `seq`
//! order. An op whose `before` does not match the element at its `seq` is a conflict and is
//! not applied, so rejecting an op re-checks every later op.

use crate::edit::{Docs, apply, canonical};
use anyhow::Result;
use brickie_model::change::{Entry, OpRecord, Review, State, Verdict};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Op records with the last review of each applied (`conflict` not yet set).
pub fn materialize(entries: &[Entry]) -> Vec<OpRecord> {
    let mut ops: Vec<OpRecord> = Vec::new();
    for e in entries {
        match e {
            Entry::Op(o) => ops.push((**o).clone()),
            Entry::Review(r) => {
                let Some(o) = ops.iter_mut().find(|o| o.seq == r.target) else {
                    continue;
                };
                o.state = match r.verdict {
                    Verdict::Accept | Verdict::Restore => State::Accepted,
                    Verdict::Reject | Verdict::Retract => State::Rejected,
                };
                o.review = Some(Review {
                    seq: r.seq,
                    by: r.by,
                    note: r.note.clone(),
                });
            }
        }
    }
    ops
}

/// Replay the accepted ops (and the proposed ones when `proposed`) onto the base, setting
/// `conflict` on each op that does not replay.
pub fn replay(
    base: &BTreeMap<String, Vec<u8>>,
    root: Option<PathBuf>,
    ops: &mut [OpRecord],
    proposed: bool,
) -> Result<Docs> {
    let mut docs = Docs::from_base(base, root)?;
    for o in ops.iter_mut() {
        o.conflict = None;
        let live = match o.state {
            State::Accepted => true,
            State::Proposed => proposed,
            State::Rejected => false,
        };
        if !live {
            continue;
        }
        if o.before.is_some() && canonical(&docs, &o.file, &o.key) != o.before {
            o.conflict = Some(format!("{} {} changed before op {}", o.file, o.key, o.seq));
            continue;
        }
        if let Err(e) = apply(&mut docs, &o.op) {
            o.conflict = Some(format!("{e:#}"));
        }
    }
    Ok(docs)
}
