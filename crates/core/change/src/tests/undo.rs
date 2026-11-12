//! Retract (the human's undo) and re-applying a conflicting op.

use super::*;
use crate::{State, Verdict};

#[test]
fn retract_takes_back_the_last_human_op() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "undo me"), None).unwrap();
    let agent = c.record(A, desc("b", "theirs"), None).unwrap();
    let r = c.retract(None).unwrap();
    assert_eq!((r.target, r.verdict), (1, Verdict::Retract));
    let log = c.log().unwrap();
    assert_eq!(log.ops[0].state, State::Rejected);
    assert!(!c.preview(true).unwrap().files()["brickie.toml"].contains("undo me"));
    // Nothing human is left; agent ops are rejected, not retracted.
    assert!(c.retract(None).is_err());
    assert!(c.retract(Some(1)).is_err());
    let e = c.retract(Some(agent.seq)).unwrap_err().to_string();
    assert!(e.contains("agent op"), "{e}");
    assert!(c.review(H, agent.seq, Verdict::Retract, None).is_err());
}

#[test]
fn reapply_records_a_conflicting_op_against_the_current_design() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "x"), None).unwrap();
    let theirs = c.record(A, desc("a", "y"), None).unwrap();
    c.retract(Some(1)).unwrap();
    assert!(c.log().unwrap().ops[1].conflict.is_some());
    assert!(c.reapply(1).is_err(), "op 1 is not in conflict");
    let new = c.reapply(theirs.seq).unwrap();
    assert_eq!((new.by, new.state), (H, State::Accepted));
    let log = c.log().unwrap();
    let old = log.ops.iter().find(|o| o.seq == theirs.seq).unwrap();
    assert_eq!(old.state, State::Rejected);
    let note = old.review.as_ref().unwrap().note.clone().unwrap();
    assert_eq!(note, format!("re-applied as op {}", new.seq));
    assert_eq!(c.status().unwrap().conflicts, 0);
    assert!(c.preview(false).unwrap().files()["brickie.toml"].contains("desc = \"y\""));
}
