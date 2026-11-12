//! Undo / redo over the human's own ops: what each would take back or restore (for the
//! buttons' tooltips), and how a gesture that recorded several ops is undone and redone whole.

use super::describe;
use brickie_web_api::ChangeView;
use brickie_web_api::change::{By, OpRecord, State};

/// The op undo would retract: the latest accepted human op.
pub fn undo_target(ch: &ChangeView) -> Option<&OpRecord> {
    ch.log
        .iter()
        .rev()
        .find(|o| o.by == By::Human && o.state == State::Accepted)
}

/// The op redo would restore: the top of the redo stack.
pub fn redo_target(ch: &ChangeView) -> Option<&OpRecord> {
    let seq = *ch.redo.last()?;
    ch.log.iter().find(|o| o.seq == seq)
}

fn group_of(groups: &[(u64, u64)], seq: u64) -> Option<(u64, u64)> {
    groups.iter().copied().find(|&(f, l)| f <= seq && seq <= l)
}

/// After undo retracted `seq`: retract the next op too (the rest of its gesture).
pub fn undo_more(groups: &[(u64, u64)], seq: u64) -> bool {
    group_of(groups, seq).is_some_and(|(f, _)| seq > f)
}

/// After redo restored `seq`: restore the next op too (the rest of its gesture).
pub fn redo_more(groups: &[(u64, u64)], seq: u64) -> bool {
    group_of(groups, seq).is_some_and(|(_, l)| seq < l)
}

/// `Undo: Added brick tel` / `Redo: ...` (with the gesture's op count), or why it is off.
pub fn undo_tip(word: &str, o: Option<&OpRecord>, groups: &[(u64, u64)]) -> String {
    let Some(o) = o else {
        return format!("Nothing to {}", word.to_lowercase());
    };
    let n = group_of(groups, o.seq).map_or(1, |(f, l)| l - f + 1);
    let more = if n > 1 {
        format!(" ({n} ops)")
    } else {
        String::new()
    };
    format!("{word}: {}{more}", describe(&o.op))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gestures_undo_and_redo_whole() {
        let g = [(5, 6)];
        assert!(undo_more(&g, 6));
        assert!(!undo_more(&g, 5));
        assert!(!undo_more(&g, 7));
        assert!(redo_more(&g, 5));
        assert!(!redo_more(&g, 6));
        assert!(!redo_more(&[], 5));
        assert_eq!(undo_tip("Redo", None, &g), "Nothing to redo");
    }
}
