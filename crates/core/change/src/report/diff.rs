//! The semantic diff: ops grouped by the element they edit, first `before` to last `after`.

use brickie_model::change::{DiffEntry, DiffOp, OpRecord, State};

/// Group ops by `(file, key)` in first-seen order. `before` / `after` come from the first and
/// last op that is applied (not rejected, not in conflict); a rename moves the element, so
/// later ops on the new name join the same entry.
pub(super) fn diff(ops: &[OpRecord]) -> Vec<DiffEntry> {
    let mut out: Vec<(String, DiffEntry, bool)> = Vec::new();
    for o in ops {
        let i = match out
            .iter()
            .position(|(alias, e, _)| e.file == o.file && (e.key == o.key || *alias == o.key))
        {
            Some(i) => i,
            None => {
                out.push((o.key.clone(), new_entry(o), false));
                out.len() - 1
            }
        };
        let (alias, e, started) = &mut out[i];
        e.ops.push(DiffOp {
            seq: o.seq,
            by: o.by,
            op: o.op.name().to_string(),
            state: o.state,
            note: o.note.clone(),
            review_note: o.review.as_ref().and_then(|r| r.note.clone()),
            conflict: o.conflict.clone(),
        });
        if o.state == State::Rejected || o.conflict.is_some() {
            continue;
        }
        if !*started {
            e.before = o.before.clone();
            *started = true;
        }
        e.after = o.after.clone();
        if let brickie_model::change::Op::RenameBrick { to, .. } = &o.op {
            *alias = format!("brick:{to}");
        }
    }
    out.into_iter()
        .map(|(_, mut e, started)| {
            if !started {
                e.after = e.before.clone();
            }
            e
        })
        .collect()
}

fn new_entry(o: &OpRecord) -> DiffEntry {
    DiffEntry {
        file: o.file.clone(),
        key: o.key.clone(),
        before: o.before.clone(),
        after: o.before.clone(),
        ops: Vec::new(),
    }
}
