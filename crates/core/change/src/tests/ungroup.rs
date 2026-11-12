//! `ungroup`: children lifted into the parent file, the child file left for deletion by hand.

use super::*;
use crate::PlanAction;

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
    let c = ws.open();
    assert!(c.record(H, ungroup("b"), None).is_err(), "b is a leaf");
    let r = c.record(H, ungroup("a"), None).unwrap();
    assert!(!r.after.unwrap().contains("path"));
    let pv = c.preview(false).unwrap();
    let root = &pv.files()["brickie.toml"];
    assert!(root.contains("\"path:g/x/\""), "{root}");
    assert!(root.contains("from = \"x\""), "{root}");
    assert!(!pv.files().contains_key("g/brickie.toml"));
    let cfg = pv.load_config().unwrap();
    assert!(cfg.brick("x").is_some() && cfg.brick("a").unwrap().sub.is_none());
    let plan = c.publish_plan().unwrap();
    let left = plan
        .files
        .iter()
        .find(|f| f.path == "g/brickie.toml")
        .unwrap();
    assert_eq!(left.action, PlanAction::Skip);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
}

#[test]
fn ungroup_of_a_group_made_in_the_change_drops_its_file() {
    let ws = Ws::new();
    let c = ws.open();
    let group = Op::MakeComposite {
        file: "brickie.toml".into(),
        name: "a".into(),
        path: "g".into(),
    };
    c.record(H, group, None).unwrap();
    let inner = Op::AddBrick {
        file: "g/brickie.toml".into(),
        brick: BrickSpec {
            name: "z".into(),
            ..BrickSpec::default()
        },
    };
    c.record(H, inner, None).unwrap();
    c.record(H, ungroup("a"), None).unwrap();
    let pv = c.preview(false).unwrap();
    assert!(pv.created().is_empty());
    assert!(pv.files()["brickie.toml"].contains("name = \"z\""));
    let plan = c.publish_plan().unwrap();
    assert!(plan.files.iter().all(|f| !f.path.starts_with("g/")));
}
