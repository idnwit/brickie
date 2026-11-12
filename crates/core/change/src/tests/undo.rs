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

#[test]
fn redo_restores_retracted_ops_last_first() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "one"), None).unwrap();
    c.record(H, desc("b", "two"), None).unwrap();
    c.retract(None).unwrap();
    c.retract(None).unwrap();
    assert_eq!(crate::redo_stack(&c.log().unwrap().entries), vec![2, 1]);
    let r = c.redo().unwrap();
    assert_eq!((r.target, r.verdict), (1, Verdict::Restore));
    assert_eq!(c.log().unwrap().ops[0].state, State::Accepted);
    assert!(c.preview(false).unwrap().files()["brickie.toml"].contains("one"));
    assert_eq!(c.redo().unwrap().target, 2);
    assert!(c.redo().is_err(), "nothing left to redo");
    assert!(c.review(H, 1, Verdict::Restore, None).is_err());
}

#[test]
fn a_new_human_op_clears_the_redo_stack() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "one"), None).unwrap();
    c.retract(None).unwrap();
    c.record(H, desc("b", "two"), None).unwrap();
    assert!(crate::redo_stack(&c.log().unwrap().entries).is_empty());
    assert!(c.redo().is_err());
}

#[test]
fn redo_refuses_an_op_that_no_longer_applies() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "mine"), None).unwrap();
    c.retract(None).unwrap();
    let theirs = c.record(A, desc("a", "theirs"), None).unwrap();
    c.review(H, theirs.seq, Verdict::Accept, None).unwrap();
    let e = c.redo().unwrap_err().to_string();
    assert!(e.contains("cannot redo op 1"), "{e}");
    assert_eq!(c.log().unwrap().ops[0].state, State::Rejected);
}
