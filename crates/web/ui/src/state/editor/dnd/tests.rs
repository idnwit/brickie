use super::*;

fn brick(name: &str, tags: &[&str]) -> BrickView {
    BrickView {
        name: name.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        ..Default::default()
    }
}

fn row(label: &str, y: f64) -> RowOut {
    RowOut {
        y,
        h: 100.0,
        label: label.into(),
        ids: vec![],
    }
}

#[test]
fn brick_body_beats_lane_and_edge() {
    let bricks = vec![brick("a", &["lib"]), brick("b", &[])];
    let mut u = Under {
        brick: Some(1),
        new_lane: false,
        up: Some("root".into()),
        lane: Some(Some("io".into())),
    };
    let b = Carried::Brick(0);
    assert_eq!(drop_target(&bricks, b, &u), Some(MoveTarget::Into(1)));
    assert_eq!(
        drop_target(&bricks, Carried::Tile, &u),
        Some(MoveTarget::Into(1))
    );
    u.brick = None;
    assert_eq!(
        drop_target(&bricks, b, &u),
        Some(MoveTarget::Up("root".into()))
    );
    // A tile ignores the frame edge: the lane decides.
    let tile = drop_target(&bricks, Carried::Tile, &u);
    assert_eq!(tile, Some(MoveTarget::Lane(Some("io".into()))));
    u.up = None;
    assert_eq!(
        drop_target(&bricks, b, &u),
        Some(MoveTarget::Lane(Some("io".into())))
    );
    // Its own lane: only a reposition.
    u.lane = Some(Some("lib".into()));
    assert_eq!(drop_target(&bricks, b, &u), None);
    u.lane = None;
    assert_eq!(drop_target(&bricks, b, &u), None);
    u.new_lane = true;
    assert_eq!(drop_target(&bricks, b, &u), Some(MoveTarget::NewLane));
}

#[test]
fn invalid_targets_say_why() {
    let mut bricks = vec![brick("a", &[]), brick("out", &[])];
    bricks[1].ghost = true;
    let over = |i| Under {
        brick: Some(i),
        ..Default::default()
    };
    let t = drop_target(&bricks, Carried::Brick(0), &over(1));
    assert!(matches!(t, Some(MoveTarget::No(_))));
    let t = drop_target(&bricks, Carried::Brick(0), &over(0));
    assert!(matches!(t, Some(MoveTarget::No(_))));
    assert!(drop_label(&bricks, Carried::Tile, t.as_ref()).starts_with('\u{2715}'));
}

#[test]
fn lanes_and_labels() {
    let rows = vec![row("lib", 0.0), row(UNTAGGED, 100.0), row("inbound", 200.0)];
    let tags = vec!["lib".to_string()];
    assert_eq!(lane_hit(&rows, &tags, 50.0), Some(Some("lib".into())));
    assert_eq!(lane_hit(&rows, &tags, 150.0), Some(None));
    assert_eq!(lane_hit(&rows, &tags, 250.0), None);
    // Between bands: the nearest row.
    let apart = vec![row("lib", 0.0), row("io", 300.0)];
    let tags = vec!["lib".to_string(), "io".to_string()];
    assert_eq!(lane_hit(&apart, &tags, 140.0), Some(Some("lib".into())));
    assert_eq!(lane_hit(&apart, &tags, 260.0), Some(Some("io".into())));
    let f = Rect {
        x: 0.0,
        y: 0.0,
        w: 400.0,
        h: 250.0,
    };
    assert_eq!(lane_rect(&rows, &f, None).map(|r| r.y), Some(100.0));
    assert_eq!(new_lane_rect(&rows, &f).y, 308.0);
    let mut web = brick("web", &[]);
    let b = vec![web.clone()];
    let into = Some(MoveTarget::Into(0));
    assert_eq!(
        drop_label(&b, Carried::Tile, into.as_ref()),
        "into web (becomes a group)"
    );
    web.file = Some("web/brickie.toml".into());
    web.children = vec!["api".into()];
    let b = vec![web];
    assert_eq!(
        drop_label(&b, Carried::Tile, into.as_ref()),
        "into web (1 inside)"
    );
    let lane = Some(MoveTarget::Lane(Some("lib".into())));
    assert_eq!(drop_label(&b, Carried::Brick(0), lane.as_ref()), "tag: lib");
    assert_eq!(drop_label(&b, Carried::Brick(0), None), "move here");
}

#[test]
fn retag_replaces_the_first_tag() {
    let t = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(retag(&t(&["io", "x"]), Some("lib")), t(&["lib", "x"]));
    assert_eq!(retag(&t(&["io", "lib"]), Some("lib")), t(&["lib"]));
    assert_eq!(retag(&t(&[]), Some("lib")), t(&["lib"]));
    assert_eq!(retag(&t(&["io", "x"]), None), t(&["x"]));
    let b = brick("a", &["lib"]);
    assert!(retag_op("f", &b, Some("lib")).is_none());
    let op = retag_op("f", &b, None).unwrap();
    let v = serde_json::to_value(&op).unwrap();
    assert_eq!(v["tags"], serde_json::json!([]));
}

#[test]
fn tile_into_a_leaf_makes_it_a_group_first() {
    let leaf = brick("web", &[]);
    let spec = tile_spec(BrickKind::Crate, " api ", "", vec![]);
    let ops = add_inside_ops("crates/brickie.toml", &leaf, spec.clone());
    assert_eq!(ops.len(), 2);
    assert!(matches!(&ops[0], Op::MakeComposite { path, .. } if path == "web"));
    assert!(
        matches!(&ops[1], Op::AddBrick { file, brick } if file == "crates/web/brickie.toml" && brick.name == "api")
    );
    assert_eq!(child_file("brickie.toml", &leaf), "web/brickie.toml");
    let mut group = leaf;
    group.file = Some("g/brickie.toml".into());
    let ops = add_inside_ops("brickie.toml", &group, spec);
    assert!(matches!(&ops[..], [Op::AddBrick { file, .. }] if file == "g/brickie.toml"));
}

#[test]
fn a_lifted_brick_goes_into_a_brick_up_or_onto_this_level() {
    let bricks = vec![brick("a", &["lib"])];
    let mut u = Under {
        brick: Some(0),
        new_lane: true,
        up: Some("root".into()),
        lane: Some(Some("lib".into())),
    };
    let l = Carried::Lifted;
    assert_eq!(drop_target(&bricks, l, &u), Some(MoveTarget::Into(0)));
    u.brick = None;
    assert_eq!(
        drop_target(&bricks, l, &u),
        Some(MoveTarget::Up("root".into()))
    );
    u.up = None;
    assert_eq!(drop_target(&bricks, l, &u), None);
    assert_eq!(drop_label(&bricks, l, None), "move here");
}
