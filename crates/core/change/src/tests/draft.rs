//! The preview is a draft: a design that is merely unfinished (a composite whose bricks do not
//! claim all its modules yet) still renders; strict `check` and publish refuse it.

use super::drift::cache;
use super::*;
use crate::TaskKind;
use brickie_model::codes::{CHANGE_INVALID, CONFIG_UNASSIGNED};

/// Leaf `a` (module pa) becomes a composite with one child that claims nothing yet.
fn unfinished(ws: &Ws) -> Change {
    cache(ws, &ws.dir.path().display().to_string());
    let c = ws.open();
    let group = Op::MakeComposite {
        file: "brickie.toml".into(),
        name: "a".into(),
        path: "a".into(),
    };
    c.record(H, group, None).unwrap();
    let child = Op::AddBrick {
        file: "a/brickie.toml".into(),
        brick: BrickSpec {
            name: "host".into(),
            ..BrickSpec::default()
        },
    };
    c.record(H, child, None).unwrap();
    c
}

#[test]
fn draft_resolves_unassigned_while_strict_refuses() {
    let ws = Ws::new();
    let c = unfinished(&ws);
    let pv = c.preview(true).unwrap();
    let l = pv.check().expect("the draft renders");
    assert_eq!(l.resolved.unassigned(&["a".to_string()]), vec!["pa"]);
    assert!(l.resolved.path("pa").is_none());
    let strict = brickie_check::check::load_from(&pv, pv.config_path());
    let err = format!("{:#}", strict.err().expect("strict check refuses"));
    assert!(err.starts_with(CONFIG_UNASSIGNED), "{err}");
    let todo = c.todo().unwrap();
    assert!(
        todo.iter()
            .any(|t| t.kind == TaskKind::Config && t.text.starts_with(CONFIG_UNASSIGNED)),
        "{todo:?}"
    );
}

#[test]
fn publish_refuses_an_unfinished_design() {
    let ws = Ws::new();
    let c = unfinished(&ws);
    let blockers = c.publish_plan().unwrap().blockers;
    let want = format!("{CHANGE_INVALID}: {CONFIG_UNASSIGNED}");
    assert!(
        blockers.iter().any(|b| b.starts_with(&want)),
        "{blockers:?}"
    );
    assert!(c.publish().is_err());
    let set = Op::SetBrick {
        file: "a/brickie.toml".into(),
        name: "host".into(),
        desc: None,
        tags: None,
        members: Some(vec!["pa".into()]),
        path: None,
    };
    let c = Change::load(&ws.config()).unwrap();
    c.record(H, set, None).unwrap();
    let blockers = c.publish_plan().unwrap().blockers;
    assert!(
        !blockers.iter().any(|b| b.contains(CONFIG_UNASSIGNED)),
        "{blockers:?}"
    );
}
