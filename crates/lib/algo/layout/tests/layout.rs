//! End-to-end tests of `layout` on a module-level fixture.

mod common;

use brickie_layout::{
    H, InBrick, InEdge, Input, Kind, Layout, Pt, Rect, Route, layout, score_layout,
};
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
            min_w: None,
            min_h: None,
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

/// True when segment `p..q` intersects the interior of axis-aligned box `r`.
fn seg_hits_rect(p: Pt, q: Pt, r: &Rect) -> bool {
    let (x0, x1, y0, y1) = (r.x, r.x + r.w, r.y, r.y + r.h);
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (num, den) in [
        (x0 - p.x, dx),
        (p.x - x1, -dx),
        (y0 - p.y, dy),
        (p.y - y1, -dy),
    ] {
        if den.abs() < 1e-9 {
            if num >= 0.0 {
                return false;
            }
        } else {
            let t = num / den;
            if den > 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 >= t1 {
                return false;
            }
        }
    }
    t0 < t1
}

fn assert_no_box_intersections(inp: &Input, l: &Layout) {
    let all: Vec<usize> = (0..inp.edges.len()).collect();
    let m = score_layout(l, &all);
    assert_eq!(m.box_hits, 0, "{m:?}");
    for (e, edge) in inp.edges.iter().enumerate() {
        let Some(route) = &l.routes[e] else {
            continue;
        };
        let pts = route.sample(2.0);
        for (bi, rect) in l.pos.iter().enumerate() {
            if bi == edge.a || bi == edge.b {
                continue;
            }
            for seg in pts.windows(2) {
                assert!(
                    !seg_hits_rect(seg[0], seg[1], rect),
                    "edge {} ({}->{}) segment {:?}..{:?} intersects box {bi} ({rect:?})",
                    edge.id,
                    edge.a,
                    edge.b,
                    seg[0],
                    seg[1]
                );
            }
        }
    }
}

fn brick(id: &str, row: usize) -> InBrick {
    InBrick {
        id: id.into(),
        row,
        min_w: None,
        min_h: None,
    }
}

fn group_brick(id: &str, row: usize, min_w: f64, min_h: f64) -> InBrick {
    InBrick {
        id: id.into(),
        row,
        min_w: Some(min_w),
        min_h: Some(min_h),
    }
}

#[test]
fn mixed_height_row_routes_without_box_intersections() {
    let inp = Input {
        rows: vec!["top".into(), "mid".into(), "bot".into()],
        bricks: vec![
            brick("api", 0),
            brick("left", 1),
            group_brick("services", 1, 360.0, 180.0),
            brick("right", 1),
            brick("store", 2),
        ],
        edges: vec![
            InEdge {
                id: "api|left".into(),
                a: 0,
                b: 1,
            },
            InEdge {
                id: "api|services".into(),
                a: 0,
                b: 2,
            },
            InEdge {
                id: "api|right".into(),
                a: 0,
                b: 3,
            },
            InEdge {
                id: "left|store".into(),
                a: 1,
                b: 4,
            },
            InEdge {
                id: "services|store".into(),
                a: 2,
                b: 4,
            },
            InEdge {
                id: "right|store".into(),
                a: 3,
                b: 4,
            },
            InEdge {
                id: "api|store".into(),
                a: 0,
                b: 4,
            },
        ],
    };
    let l = layout(&inp);
    // Group box honours min_w/min_h; normal bricks keep height H.
    assert!(l.pos[2].w >= 360.0);
    assert_eq!(l.pos[2].h, 180.0);
    assert_eq!(l.pos[1].h, H);
    assert_eq!(l.pos[3].h, H);
    assert_eq!(l.rows[1].h, 180.0);
    // Adjacent 1-brick rows are not stretched to 360px when only connected to narrow targets;
    // here api/store connect across the full row, so test a clean adjacent 1-brick row below.
    assert_no_box_intersections(&inp, &l);
}

#[test]
fn adjacent_single_brick_rows_not_stretched_by_wide_group() {
    // 1-brick rows above and below a mixed row (normal + 360x180 group) and a lone 420x180 group.
    let inp_mixed = Input {
        rows: vec!["r0".into(), "r1".into(), "r2".into()],
        bricks: vec![
            brick("top", 0),
            brick("left", 1),
            group_brick("services", 1, 360.0, 180.0),
            brick("bot", 2),
        ],
        edges: vec![
            InEdge {
                id: "top|left".into(),
                a: 0,
                b: 1,
            },
            InEdge {
                id: "left|bot".into(),
                a: 1,
                b: 3,
            },
            InEdge {
                id: "services|bot".into(),
                a: 2,
                b: 3,
            },
        ],
    };
    let l_mixed = layout(&inp_mixed);
    assert!(l_mixed.pos[0].w < 360.0, "top w={}", l_mixed.pos[0].w);
    assert!(l_mixed.pos[3].w < 360.0, "bot w={}", l_mixed.pos[3].w);
    assert_no_box_intersections(&inp_mixed, &l_mixed);

    let inp_wide = Input {
        rows: vec!["r0".into(), "r1".into(), "r2".into()],
        bricks: vec![
            brick("top", 0),
            group_brick("services", 1, 420.0, 180.0),
            brick("bot", 2),
        ],
        edges: vec![
            InEdge {
                id: "top|services".into(),
                a: 0,
                b: 1,
            },
            InEdge {
                id: "services|bot".into(),
                a: 1,
                b: 2,
            },
        ],
    };
    let l_wide = layout(&inp_wide);
    assert!(l_wide.pos[1].w >= 420.0);
    assert!(l_wide.pos[0].w < 360.0, "top w={}", l_wide.pos[0].w);
    assert!(l_wide.pos[2].w < 360.0, "bot w={}", l_wide.pos[2].w);
    assert_no_box_intersections(&inp_wide, &l_wide);
}

#[test]
fn bottom_arc_clears_taller_sibling_in_same_row() {
    let inp = Input {
        rows: vec!["mid".into()],
        bricks: vec![
            brick("left", 0),
            group_brick("services", 0, 360.0, 180.0),
            brick("right", 0),
        ],
        edges: vec![
            InEdge {
                id: "left|right|a".into(),
                a: 0,
                b: 2,
            },
            InEdge {
                id: "right|left|b".into(),
                a: 2,
                b: 0,
            },
        ],
    };
    let l = layout(&inp);
    assert!(
        l.kinds
            .iter()
            .any(|k| matches!(k, Some(Kind::Arc { top: false, .. }))),
        "expected at least one bottom arc, got {:?}",
        l.kinds
    );
    assert_no_box_intersections(&inp, &l);
}
