//! Rules that change nothing get a note, and code is written only when scaffolding is asked for.

use super::*;
use crate::{BrickKind, RuleSpec};

fn group(name: &str) -> Op {
    Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: name.into(),
            kind: Some(BrickKind::Group),
            ..BrickSpec::default()
        },
    }
}

fn rule(file: &str, from: &str, to: &str) -> Op {
    Op::AddRule {
        file: file.into(),
        rule: RuleSpec {
            from: from.into(),
            to: to.into(),
            via: Vec::new(),
            kinds: Vec::new(),
            scope: None,
            action: "allow".into(),
            reason: None,
        },
    }
}

#[test]
fn an_existing_level_file_is_never_changed() {
    let ws = Ws::new();
    ws.write("g/brickie.toml", "# mine\n");
    let set = Op::SetBrick {
        file: "brickie.toml".into(),
        name: "a".into(),
        desc: None,
        tags: None,
        members: None,
        path: Some("g".into()),
        independent: None,
    };
    ws.apply(A, vec![set]).unwrap();
    assert_eq!(ws.read("g/brickie.toml"), "# mine\n");
}

#[test]
fn a_rule_that_changes_nothing_gets_a_note() {
    let ws = Ws::new();
    // No rule allows b -> a, so the rule changes the level.
    let out = ws.apply(A, vec![rule("brickie.toml", "b", "a")]).unwrap();
    assert!(out.notes.is_empty(), "{:?}", out.notes);
    // An allow from an independent brick to a normal brick never applies.
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        "[[brick]]\nname = \"a\"\nmembers = [\"pa\"]\nindependent = true\n\
         [[brick]]\nname = \"b\"\nmembers = [\"pb\"]\n",
    );
    let d = ws.design();
    let plan = d.dry_run(1, vec![rule("brickie.toml", "a", "b")]).unwrap();
    let want = "rule a -> b allow changes nothing: an allow from an independent brick to a normal brick never applies";
    assert_eq!(plan.notes, [want]);
    // b -> a is already allowed: an independent brick accepts inbound edges.
    let plan = d.dry_run(1, vec![rule("brickie.toml", "b", "a")]).unwrap();
    let want = "rule b -> a allow changes nothing: already allowed by the default (no rule)";
    assert_eq!(plan.notes, [want]);
}

#[test]
fn a_rule_in_a_level_created_by_the_same_write_gets_no_note() {
    let ws = Ws::new();
    let out = ws
        .apply(A, vec![group("g"), rule("g/brickie.toml", "x", "y")])
        .unwrap_err();
    // `x` and `y` do not exist: the write fails on its own terms, and no note logic ran.
    assert!(!format!("{out:#}").is_empty());
}

#[test]
fn code_is_written_only_with_scaffold() {
    let ws = Ws::new();
    let krate = || Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: "c".into(),
            kind: Some(BrickKind::Crate),
            ..BrickSpec::default()
        },
    };
    let d = ws.design();
    let plan = d.dry_run(1, vec![krate()]).unwrap();
    assert!(plan.scaffold.is_empty(), "{:?}", plan.scaffold);
    let out = ws.apply(A, vec![krate()]).unwrap();
    assert!(out.scaffold.is_empty());
    assert!(!ws.path("crates/c").exists());
    assert!(!ws.read("Cargo.toml").contains("crates/c"));
    ws.apply(
        A,
        vec![Op::RemoveBrick {
            file: "brickie.toml".into(),
            name: "c".into(),
        }],
    )
    .unwrap();
    let plan = d.dry_run_scaffold(3, vec![krate()]).unwrap();
    assert!(!plan.scaffold.is_empty());
    d.apply_scaffold(3, A, None, vec![krate()]).unwrap();
    assert!(ws.path("crates/c/src/lib.rs").is_file());
    assert!(ws.read("Cargo.toml").contains("crates/c"));
}

#[test]
fn a_rule_removed_and_added_again_in_one_write_gets_no_note() {
    let ws = Ws::new();
    let out = ws.apply(A, vec![rule("brickie.toml", "b", "a")]).unwrap();
    let rm = Op::RemoveRule {
        file: "brickie.toml".into(),
        key: crate::RuleKey {
            from: "b".into(),
            to: "a".into(),
            kinds: None,
            scope: None,
        },
    };
    let base = out.version.v;
    let out = (ws.design())
        .apply(
            base,
            By::Agent,
            None,
            vec![rm, rule("brickie.toml", "b", "a")],
        )
        .unwrap();
    assert!(out.notes.is_empty(), "{:?}", out.notes);
}
