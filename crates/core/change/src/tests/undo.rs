//! `undo`: a version's file changes reverted as a new version, bytes and comments included.

use super::*;

#[test]
fn undo_restores_bytes_and_comments() {
    let ws = Ws::new();
    let before = ws.read("brickie.toml");
    ws.apply(
        A,
        vec![
            Op::RemoveBrick {
                file: "brickie.toml".into(),
                name: "b".into(),
            },
            Op::RemoveRule {
                file: "brickie.toml".into(),
                key: crate::RuleKey {
                    from: "a".into(),
                    to: "b".into(),
                    ..Default::default()
                },
            },
        ],
    )
    .unwrap();
    assert_ne!(ws.read("brickie.toml"), before);
    let d = ws.design();
    let out = d.undo(2, A, None).unwrap();
    assert_eq!(out.version.v, 3);
    assert_eq!(out.version.note.as_deref(), Some("undo v2"));
    assert_eq!(ws.read("brickie.toml"), before);
}

#[test]
fn undo_removes_a_created_file() {
    let ws = Ws::new();
    let group = Op::MakeComposite {
        file: "brickie.toml".into(),
        name: "a".into(),
        path: "g".into(),
    };
    ws.apply(A, vec![group]).unwrap();
    assert!(ws.path("g/brickie.toml").exists());
    ws.design().undo(2, A, None).unwrap();
    assert!(!ws.path("g/brickie.toml").exists());
}

#[test]
fn an_agent_never_undoes_a_human_version() {
    let ws = Ws::new();
    ws.apply(H, vec![desc("a", "x")]).unwrap();
    let e = ws.design().undo(2, A, None).unwrap_err().to_string();
    assert!(e.contains("made by human"), "{e}");
    ws.design().undo(2, H, None).unwrap();
}

#[test]
fn undo_of_an_older_version_refuses_when_a_later_one_touched_its_files() {
    let ws = Ws::new();
    ws.apply(A, vec![desc("a", "x")]).unwrap();
    ws.apply(A, vec![desc("b", "y")]).unwrap();
    let e = ws.design().undo(3, A, Some(2)).unwrap_err().to_string();
    assert!(e.contains("v3 changed brickie.toml after v2"), "{e}");
    let first = ws.design().undo(3, A, Some(1)).unwrap_err().to_string();
    assert!(first.contains("oldest kept version"), "{first}");
}

#[test]
fn undo_goes_back_step_by_step_once_the_later_versions_are_reverted() {
    let ws = Ws::new();
    ws.apply(H, vec![desc("a", "x")]).unwrap();
    ws.apply(H, vec![desc("b", "y")]).unwrap();
    let before = std::fs::read_to_string(ws.path("brickie.toml")).unwrap();
    ws.design().undo(3, H, Some(3)).unwrap();
    // v3 is reverted by v4: the file is as v2 left it, so v2 can be undone too.
    ws.design().undo(4, H, Some(2)).unwrap();
    let now = std::fs::read_to_string(ws.path("brickie.toml")).unwrap();
    assert!(
        !now.contains("desc = \"x\"") && !now.contains("desc = \"y\""),
        "{now}"
    );
    assert_ne!(now, before);
}
