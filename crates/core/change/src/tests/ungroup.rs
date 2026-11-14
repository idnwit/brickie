//! `ungroup`: children lifted into the parent file; the child file leaves the tree but stays on
//! disk.

use super::*;

const CHILD: &str = r#"exports = ["x"]

[[brick]]
name = "x"
members = ["path:x/"]

[[brick]]
name = "y"
members = ["py"]

[[deps.rule]]
from = "x"
to = "y"
action = "allow"
"#;

fn ungroup(name: &str) -> Op {
    Op::Ungroup {
        file: "brickie.toml".into(),
        name: name.into(),
    }
}

#[test]
fn ungroup_lifts_children_and_rules() {
    let ws = Ws::new();
    ws.write(
        "brickie.toml",
        &ROOT.replace("tags = [\"core\"]", "path = \"g\""),
    );
    ws.write("g/brickie.toml", CHILD);
    assert!(ws.apply(H, vec![ungroup("b")]).is_err(), "b is a leaf");
    let out = ws.apply(H, vec![ungroup("a")]).unwrap();
    assert_eq!(out.version.removed, vec!["g/brickie.toml".to_string()]);
    let root = ws.read("brickie.toml");
    assert!(root.contains("\"path:g/x/\""), "{root}");
    assert!(root.contains("from = \"x\""), "{root}");
    assert!(ws.path("g/brickie.toml").exists());
    let cfg = ws.design().config().unwrap();
    assert!(cfg.brick("x").is_some() && cfg.brick("a").unwrap().sub.is_none());
}

#[test]
fn group_then_ungroup_in_one_write() {
    let ws = Ws::new();
    let group = Op::MakeComposite {
        file: "brickie.toml".into(),
        name: "a".into(),
        path: "g".into(),
    };
    let inner = Op::AddBrick {
        file: "g/brickie.toml".into(),
        brick: BrickSpec {
            name: "z".into(),
            ..BrickSpec::default()
        },
    };
    ws.apply(H, vec![group, inner, ungroup("a")]).unwrap();
    assert!(ws.read("brickie.toml").contains("name = \"z\""));
    assert!(!ws.path("g/brickie.toml").exists());
}

#[test]
fn group_leaves_no_self_rule() {
    let ws = Ws::new();
    let mv = |name: &str| Op::MoveBrick {
        file: "brickie.toml".into(),
        name: name.into(),
        into_file: "brickie.toml".into(),
        into: Some("g".into()),
        path: None,
    };
    let add = Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: "g".into(),
            kind: Some(crate::BrickKind::Group),
            ..BrickSpec::default()
        },
    };
    ws.apply(H, vec![add, mv("a"), mv("b")]).unwrap();
    let root = ws.read("brickie.toml");
    assert!(!root.contains("[[deps.rule]]"), "{root}");
}

#[test]
fn group_rewrites_outside_rule() {
    let ws = Ws::new();
    let add = Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: "g".into(),
            kind: Some(crate::BrickKind::Group),
            ..BrickSpec::default()
        },
    };
    let mv = Op::MoveBrick {
        file: "brickie.toml".into(),
        name: "a".into(),
        into_file: "brickie.toml".into(),
        into: Some("g".into()),
        path: None,
    };
    ws.apply(H, vec![add, mv]).unwrap();
    let root = ws.read("brickie.toml");
    assert!(
        root.contains("from = \"g\"") && root.contains("to = \"b\""),
        "{root}"
    );
}
