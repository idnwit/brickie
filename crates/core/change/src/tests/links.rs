//! Links are named by identity like rules; a link no rule allows is drafted with a note and
//! blocks publishing.

use super::*;
use crate::{RuleKey, RulePatch, RuleSpec};

fn spec(from: &str, to: &str) -> RuleSpec {
    RuleSpec {
        from: from.into(),
        to: to.into(),
        reason: "because".into(),
        ..Default::default()
    }
}

fn add_link(from: &str, to: &str) -> Op {
    Op::AddLink {
        file: "brickie.toml".into(),
        link: spec(from, to),
    }
}

fn key(from: &str, to: &str) -> RuleKey {
    RuleKey {
        from: from.into(),
        to: to.into(),
        ..Default::default()
    }
}

#[test]
fn add_set_and_remove_a_link_by_identity() {
    let ws = Ws::new();
    ws.apply(A, vec![add_link("a", "b")]).unwrap();
    assert!(ws.read("brickie.toml").contains("[[link]]"));
    let e = format!("{:#}", ws.apply(A, vec![add_link("a", "b")]).unwrap_err());
    assert!(e.contains("design/exists"), "{e}");
    let set = Op::SetLink {
        file: "brickie.toml".into(),
        key: key("a", "b"),
        set: RulePatch {
            reason: Some("newer".into()),
        },
    };
    ws.apply(A, vec![set]).unwrap();
    let text = ws.read("brickie.toml");
    assert!(
        text.contains("reason = \"newer\"") && !text.contains("because"),
        "{text}"
    );
    let rm = || Op::RemoveLink {
        file: "brickie.toml".into(),
        key: key("a", "b"),
    };
    ws.apply(A, vec![rm()]).unwrap();
    assert!(!ws.read("brickie.toml").contains("[[link]]"));
    let e = format!("{:#}", ws.apply(A, vec![rm()]).unwrap_err());
    assert!(e.contains("design/no-match"), "{e}");
}

#[test]
fn a_link_a_rule_allows_gets_no_note() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![add_link("a", "b")]).unwrap();
    assert!(out.notes.is_empty(), "{:?}", out.notes);
}

#[test]
fn a_link_no_rule_allows_is_drafted_with_a_note() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![add_link("b", "a")]).unwrap();
    assert!(
        out.notes.iter().any(|n| n.contains("design/link-denied")),
        "{:?}",
        out.notes
    );
    assert!(ws.read("brickie.toml").contains("[[link]]"));
}
