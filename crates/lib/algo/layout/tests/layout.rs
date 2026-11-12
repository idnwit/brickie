//! End-to-end tests of `layout` on a module-level fixture.

mod common;

use brickie_layout::{InBrick, Input, Layout, Route, layout, score_layout};
use common::fixture;

#[test]
fn no_edge_crosses_a_box() {
    let l = layout(&fixture());
    let all: Vec<usize> = (0..fixture().edges.len()).collect();
    let m = score_layout(&l, &all);
    assert_eq!(m.box_hits, 0, "{m:?}");
    assert!(l.routes.iter().all(Option::is_some));
}

#[test]
fn boxes_do_not_overlap() {
    let l = layout(&fixture());
    for (i, a) in l.pos.iter().enumerate() {
        for b in &l.pos[i + 1..] {
            let apart =
                a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn layout_is_deterministic() {
    let (a, b) = (layout(&fixture()), layout(&fixture()));
    assert_eq!(a.pos, b.pos);
    let d = |l: &Layout| {
        l.routes
            .iter()
            .map(|r| r.as_ref().map(Route::d))
            .collect::<Vec<_>>()
    };
    assert_eq!(d(&a), d(&b));
    assert_eq!(a.cross, b.cross);
}

#[test]
fn crossings_at_most_the_config_order() {
    let l = layout(&fixture());
    assert!(l.cross <= l.cross0, "{} > {}", l.cross, l.cross0);
}

#[test]
fn rows_follow_input_rows() {
    let l = layout(&fixture());
    let y = |i: usize| l.pos[i].y;
    // contract (row 0) above engine (row 1) above alsa (row 2) above client (row 3)
    assert!(y(0) < y(2) && y(2) < y(6) && y(6) < y(11));
    assert_eq!(l.rows.len(), 4);
}

#[test]
fn empty_and_single_brick_graphs() {
    let l = layout(&Input::default());
    assert!(l.pos.is_empty());
    let one = Input {
        rows: vec!["a".into()],
        bricks: vec![InBrick {
            id: "x".into(),
            row: 0,
        }],
        edges: vec![],
    };
    assert_eq!(layout(&one).pos.len(), 1);
}

#[test]
fn dragged_positions_reroute_without_panic() {
    let inp = fixture();
    let l = layout(&inp);
    let mut p = l.pos.clone();
    p[3].x += 40.0;
    p[3].y += 15.0;
    for e in 0..inp.edges.len() {
        assert!(l.path(e, &p).is_some());
    }
}
