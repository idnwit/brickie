use super::*;
use crate::{BrickKind, LayoutHint, PlanAction};

fn kind(name: &str, k: BrickKind) -> Op {
    Op::AddBrick {
        file: "brickie.toml".into(),
        brick: BrickSpec {
            name: name.into(),
            kind: Some(k),
            ..BrickSpec::default()
        },
    }
}

fn p(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

#[test]
fn kinds_fill_in_members_and_groups() {
    let ws = Ws::new();
    let c = ws.open();
    let r = c.record(H, kind("store", BrickKind::Crate), None).unwrap();
    assert!(r.after.unwrap().contains("path:crates/store/"));
    c.record(H, kind("ui", BrickKind::Group), None).unwrap();
    let pv = c.preview(false).unwrap();
    assert!(pv.created().contains("ui/brickie.toml"));
    assert!(pv.load_config().unwrap().brick("ui").unwrap().sub.is_some());
    assert!(!pv.files()["brickie.toml"].contains("kind"));
}

#[test]
fn move_into_a_sibling_makes_it_composite_and_lifts_rules() {
    let ws = Ws::new();
    let c = ws.open();
    let op = c.move_op(&p(&["b"]), &p(&["a"])).unwrap();
    let r = c.record(H, op, None).unwrap();
    assert!(r.after.as_ref().unwrap().contains("name = \"b\""), "{r:?}");
    let pv = c.preview(false).unwrap();
    let root = &pv.files()["brickie.toml"];
    assert!(
        root.contains("path = \"a\"") && root.contains("to = \"a\""),
        "{root}"
    );
    assert!(pv.files()["a/brickie.toml"].contains("name = \"b\""));
    let cfg = pv.load_config().unwrap();
    assert!(cfg.brick_at(&p(&["a", "b"])).is_some());
    // And back up to the root.
    let back = c.move_op(&p(&["a", "b"]), &[]).unwrap();
    c.record(H, back, None).unwrap();
    let cfg = c.preview(false).unwrap().load_config().unwrap();
    assert!(cfg.brick("b").is_some());
    let plan = c.publish_plan().unwrap();
    let manual = plan
        .files
        .iter()
        .find(|f| f.action == PlanAction::Skip)
        .unwrap();
    assert_eq!(manual.path, "brick:b");
}

#[test]
fn move_rebases_path_members() {
    let ws = Ws::new();
    let c = ws.open();
    c.record(H, kind("store", BrickKind::Crate), None).unwrap();
    c.record(H, kind("grp", BrickKind::Group), None).unwrap();
    let op = c.move_op(&p(&["store"]), &p(&["grp"])).unwrap();
    c.record(H, op, None).unwrap();
    let text = c.preview(false).unwrap().files()["grp/brickie.toml"].clone();
    assert!(text.contains("path:../crates/store/"), "{text}");
}

#[test]
fn link_options_follow_the_rules() {
    let ws = Ws::new();
    let c = ws.open();
    let opts = c.link_options(&p(&["a"])).unwrap();
    assert_eq!(opts.len(), 6);
    let prod_uses = opts
        .iter()
        .find(|o| o.kind == "uses" && o.scope == "prod")
        .unwrap();
    assert_eq!(
        (prod_uses.to.as_str(), prod_uses.action.as_str()),
        ("b", "allow")
    );
    assert!(prod_uses.rule.is_some());
    let test = opts.iter().find(|o| o.scope == "test").unwrap();
    assert_eq!((test.action.as_str(), test.rule.as_ref()), ("deny", None));
}

#[test]
fn layout_hints_persist() {
    let ws = Ws::new();
    let c = ws.open();
    c.set_layout("root/a", Some(LayoutHint { x: 1.0, y: 2.5 }))
        .unwrap();
    c.set_layout("root/b", Some(LayoutHint { x: 0.0, y: 0.0 }))
        .unwrap();
    c.set_layout("root/b", None).unwrap();
    let l = ws.open_existing().layout().unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l["root/a"], LayoutHint { x: 1.0, y: 2.5 });
}
