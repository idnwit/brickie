use super::*;
use crate::{Entry, Op, RuleSpec, State, Verdict};

#[test]
fn human_ops_are_accepted_and_only_previewed() {
    let ws = Ws::new();
    let c = ws.open();
    let r = c.record(H, add("c", &["pc"]), Some("new".into())).unwrap();
    assert_eq!(
        (r.seq, r.state, r.key.as_str()),
        (1, State::Accepted, "brick:c")
    );
    assert_eq!(r.before, None);
    assert!(r.after.unwrap().contains("name = \"c\""));
    // Nobody writes the toml while the change is open.
    assert_eq!(ws.read("brickie.toml"), ROOT);
    assert_eq!(c.intent().unwrap(), "test change");
    assert_eq!(c.config_path(), c.root().join("brickie.toml"));
    let p = c.preview(false).unwrap();
    assert_eq!(p.config_path(), c.config_path());
    let text = &p.files()["brickie.toml"];
    assert!(
        text.contains("name = \"c\"") && text.contains("# first"),
        "{text}"
    );
    let cfg = p.load_config().unwrap();
    assert!(cfg.brick("c").is_some());
    // A second change cannot be opened.
    let e = Change::open(&ws.config(), "x").err().unwrap().to_string();
    assert!(e.starts_with("change/open"), "{e}");
}

#[test]
fn agent_ops_wait_for_review_and_reject_reverts_them() {
    let ws = Ws::new();
    let c = ws.open();
    let o = c.record(A, desc("a", "core logic"), None).unwrap();
    assert_eq!(o.state, State::Proposed);
    assert!(!c.preview(false).unwrap().files()["brickie.toml"].contains("core logic"));
    assert!(c.preview(true).unwrap().files()["brickie.toml"].contains("core logic"));
    assert!(
        c.review(A, o.seq, Verdict::Accept, None).is_err(),
        "agents do not review"
    );
    let r = c
        .review(H, o.seq, Verdict::Reject, Some("too vague".into()))
        .unwrap();
    assert_eq!(r.seq, 2);
    let log = c.log().unwrap();
    assert_eq!(log.ops[0].state, State::Rejected);
    assert_eq!(
        log.ops[0].review.as_ref().unwrap().note.as_deref(),
        Some("too vague")
    );
    assert!(!c.preview(true).unwrap().files()["brickie.toml"].contains("core logic"));
    assert!(c.review(H, o.seq, Verdict::Reject, None).is_err());
}

#[test]
fn rejecting_an_op_puts_later_ops_on_it_in_conflict() {
    let ws = Ws::new();
    let c = ws.open();
    let p = c.record(A, desc("a", "v1"), None).unwrap();
    // The human edits the same brick on top of the proposal.
    let h = c.record(H, desc("a", "v2"), None).unwrap();
    assert!(h.before.as_ref().unwrap().contains("v1"));
    let unrelated = c.record(H, desc("b", "other"), None).unwrap();
    assert!(c.log().unwrap().ops.iter().all(|o| o.conflict.is_none()));
    c.review(H, p.seq, Verdict::Reject, Some("no".into()))
        .unwrap();
    let log = c.log().unwrap();
    let get = |s: u64| log.ops.iter().find(|o| o.seq == s).unwrap();
    assert!(
        get(h.seq).conflict.is_some(),
        "cascade: later op no longer replays"
    );
    assert!(get(unrelated.seq).conflict.is_none());
    let todo = c.todo().unwrap();
    assert!(todo.iter().any(|t| t.kind == crate::TaskKind::Conflict));
    assert!(todo.iter().any(|t| t.text.contains("rejected: no")));
}

#[test]
fn an_op_that_does_not_apply_is_not_recorded() {
    let ws = Ws::new();
    let c = ws.open();
    assert!(c.record(H, add("a", &[]), None).is_err(), "duplicate");
    assert!(c.record(H, desc("ghost", "x"), None).is_err());
    assert!(c.log().unwrap().entries.is_empty());
}

#[test]
fn rename_rewrites_selectors_and_exports() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace(
            "exports = []",
            "exports = [\"b\", { brick = \"a\", scope = \"test\" }]",
        ),
    );
    let c = ws.open();
    let rename = Op::RenameBrick {
        file: "brickie.toml".into(),
        from: "a".into(),
        to: "core".into(),
    };
    let r = c.record(H, rename, None).unwrap();
    assert!(r.after.unwrap().contains("name = \"core\""));
    let text = c.preview(false).unwrap().files()["brickie.toml"].clone();
    assert!(text.contains("from = \"core\""), "{text}");
    assert!(text.contains("brick = \"core\""), "{text}");
    assert!(!text.contains("\"a\""), "{text}");
    // Later ops on the new name coalesce into one diff entry.
    c.record(H, desc("core", "the core"), None).unwrap();
    let diff = c.diff().unwrap();
    assert_eq!(diff.len(), 1, "{diff:?}");
    assert!(diff[0].after.as_ref().unwrap().contains("the core"));
}

#[test]
fn rules_by_index_and_composites() {
    let ws = Ws::new();
    let c = ws.open();
    let rule = RuleSpec {
        from: "b".into(),
        to: "a".into(),
        action: "deny".into(),
        reason: Some("layering".into()),
        ..RuleSpec::default()
    };
    let r = c
        .record(
            H,
            Op::AddRule {
                file: "brickie.toml".into(),
                rule,
            },
            None,
        )
        .unwrap();
    assert_eq!(r.key, "rule:1");
    let rm = Op::RemoveRule {
        file: "brickie.toml".into(),
        index: 0,
    };
    let r = c.record(H, rm, None).unwrap();
    assert!(r.before.unwrap().contains("allow") && r.after.is_none());
    let mk = Op::MakeComposite {
        file: "brickie.toml".into(),
        name: "b".into(),
        path: "crates/b".into(),
    };
    c.record(H, mk, None).unwrap();
    c.record(
        H,
        Op::AddBrick {
            file: "crates/b/brickie.toml".into(),
            brick: crate::BrickSpec {
                name: "inner".into(),
                members: vec!["pb".into()],
                ..Default::default()
            },
        },
        None,
    )
    .unwrap();
    let p = c.preview(false).unwrap();
    assert!(p.created().contains("crates/b/brickie.toml"));
    let cfg = p.load_config().unwrap();
    assert_eq!(cfg.deps.rules.len(), 1);
    let sub = cfg.node(&["b".into()]).unwrap();
    assert!(sub.brick("inner").is_some());
}

#[test]
fn readers_see_what_is_new_once() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, desc("a", "x"), None).unwrap();
    let s = c.since(A, None).unwrap();
    assert_eq!((s.from, s.head, s.entries.len()), (0, 1, 1));
    assert!(matches!(s.entries[0], Entry::Op(_)));
    assert!(c.since(A, None).unwrap().entries.is_empty());
    c.record(H, desc("b", "y"), None).unwrap();
    assert_eq!(c.since(A, None).unwrap().entries.len(), 1);
    // An explicit cursor does not move the stored one.
    assert_eq!(c.since(H, Some(0)).unwrap().entries.len(), 2);
    assert_eq!(c.since(H, None).unwrap().entries.len(), 2);
}
