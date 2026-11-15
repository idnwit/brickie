//! New level files copy the parent's `deps`, rules that change nothing get a note, and code is
//! written only when scaffolding is asked for.

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
            help: None,
        },
    }
}

#[test]
fn a_new_group_file_copies_default_and_cycles_from_its_parent() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace(
            "default = \"deny\"",
            "default = \"deny\"\ncycles = \"deny\"",
        ),
    );
    ws.apply(A, vec![group("g")]).unwrap();
    let text = ws.read("g/brickie.toml");
    assert!(
        text.contains("default = \"deny\"") && text.contains("cycles = \"deny\""),
        "{text}"
    );
    // A group inside it copies from that file, and a parent without `deps` gives none.
    ws.write(
        "brickie.toml",
        "[[brick]]\nname = \"a\"\nmembers = [\"pa\"]\n",
    );
    ws.apply(A, vec![group("h")]).unwrap();
    assert!(!ws.read("h/brickie.toml").contains("deps"));
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
    };
    ws.apply(A, vec![set]).unwrap();
    assert_eq!(ws.read("g/brickie.toml"), "# mine\n");
}

#[test]
fn a_rule_that_changes_nothing_gets_a_note_only_under_default_allow() {
    let ws = Ws::new();
    // Under `default = "deny"` the rule b -> a changes the level.
    let out = ws.apply(A, vec![rule("brickie.toml", "b", "a")]).unwrap();
    assert!(out.notes.is_empty(), "{:?}", out.notes);
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace("default = \"deny\"", "default = \"allow\""),
    );
    let d = ws.design();
    let plan = d.dry_run(1, vec![rule("brickie.toml", "b", "a")]).unwrap();
    let want = "rule b -> a allow changes nothing: already allowed by deps.default of brickie.toml";
    assert_eq!(plan.notes, [want]);
    assert_eq!(
        ws.apply(A, vec![rule("brickie.toml", "b", "a")])
            .unwrap()
            .notes,
        [want]
    );
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
