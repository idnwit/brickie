//! Characterization tests for `measure_items`: golden values from the current implementation.

use crate::measure::{Item, Metrics, mk_item};
use crate::*;

fn p(x: f64, y: f64) -> Pt {
    Pt { x, y }
}

fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn item(a: usize, b: usize, pts: &[(f64, f64)]) -> Item {
    mk_item(a, b, pts.iter().map(|&(x, y)| p(x, y)).collect())
}

fn m(edges: usize, crossings: usize, box_hits: usize, bends: usize) -> Metrics {
    Metrics {
        edges,
        crossings,
        box_hits,
        bends,
    }
}

/// Four far-away boxes that never interfere unless a test puts a path over them.
fn far() -> Vec<Rect> {
    (0..4)
        .map(|i| r(1000.0 + 100.0 * i as f64, 1000.0, 20.0, 20.0))
        .collect()
}

#[test]
fn no_items() {
    assert_eq!(measure_items(&[], &far()), m(0, 0, 0, 0));
    assert_eq!(measure_items(&[], &[]), m(0, 0, 0, 0));
}

#[test]
fn single_item_has_no_crossings() {
    let e = item(0, 1, &[(0.0, 0.0), (10.0, 10.0)]);
    assert_eq!(measure_items(&[e], &far()), m(1, 0, 0, 0));
}

#[test]
fn two_crossing_segments() {
    let es = [
        item(0, 1, &[(0.0, 0.0), (10.0, 10.0)]),
        item(2, 3, &[(0.0, 10.0), (10.0, 0.0)]),
    ];
    assert_eq!(measure_items(&es, &far()), m(2, 1, 0, 0));
}

#[test]
fn parallel_and_touching_segments_do_not_cross() {
    let par = [
        item(0, 1, &[(0.0, 0.0), (10.0, 0.0)]),
        item(2, 3, &[(0.0, 5.0), (10.0, 5.0)]),
    ];
    assert_eq!(measure_items(&par, &far()), m(2, 0, 0, 0));
    // T junction: the endpoint lies on the other segment (strict inequality, so no crossing).
    let tee = [
        item(0, 1, &[(0.0, 0.0), (10.0, 0.0)]),
        item(2, 3, &[(5.0, 0.0), (5.0, 10.0)]),
    ];
    assert_eq!(measure_items(&tee, &far()), m(2, 0, 0, 0));
    // Collinear overlap.
    let col = [
        item(0, 1, &[(0.0, 0.0), (10.0, 0.0)]),
        item(2, 3, &[(5.0, 0.0), (15.0, 0.0)]),
    ];
    assert_eq!(measure_items(&col, &far()), m(2, 0, 0, 0));
}

#[test]
fn disjoint_bounding_boxes_skip_the_pair() {
    let es = [
        item(0, 1, &[(0.0, 0.0), (10.0, 10.0)]),
        item(2, 3, &[(20.0, 0.0), (30.0, 10.0)]),
    ];
    assert_eq!(measure_items(&es, &far()), m(2, 0, 0, 0));
}

#[test]
fn crossing_near_a_shared_node_is_skipped() {
    // Both edges end at node 0, whose box is at the crossing; the crossing starts inside the
    // box grown by 10.
    let nodes = vec![
        r(-5.0, -5.0, 10.0, 10.0),
        r(1000.0, 1000.0, 20.0, 20.0),
        r(1100.0, 1000.0, 20.0, 20.0),
    ];
    let es = [
        item(0, 1, &[(0.0, 0.0), (10.0, 10.0)]),
        item(0, 2, &[(0.0, 10.0), (10.0, 0.0)]),
    ];
    assert_eq!(measure_items(&es, &nodes), m(2, 0, 0, 0));
    // The same geometry without a shared node counts.
    let es2 = [
        item(1, 2, &[(0.0, 0.0), (10.0, 10.0)]),
        item(1, 2, &[(0.0, 10.0), (10.0, 0.0)]),
    ];
    assert_eq!(measure_items(&es2, &nodes), m(2, 1, 1, 0));
}

#[test]
fn shared_node_far_from_the_crossing_still_counts() {
    // Shared node 0 is at the origin; the crossing starts at x=40, more than 10 away from it.
    let nodes = vec![
        r(-5.0, -5.0, 10.0, 10.0),
        r(1000.0, 1000.0, 20.0, 20.0),
        r(1100.0, 1000.0, 20.0, 20.0),
    ];
    let es = [
        item(0, 1, &[(0.0, 0.0), (40.0, 0.0), (60.0, 10.0)]),
        item(0, 2, &[(50.0, -10.0), (50.0, 10.0)]),
    ];
    assert_eq!(measure_items(&es, &nodes), m(2, 1, 0, 0));
}

#[test]
fn consecutive_crossings_within_two_segments_count_once() {
    // A zig-zag crosses a straight line at segments 0, 1 and 2 (gap <= 2 skipped), then 5.
    let zig: Vec<(f64, f64)> = (0..8)
        .map(|i| (i as f64 * 10.0, if i % 2 == 0 { -5.0 } else { 5.0 }))
        .collect();
    let es = [item(0, 1, &zig), item(2, 3, &[(0.0, 0.0), (70.0, 0.0)])];
    assert_eq!(measure_items(&es, &far()), m(2, 3, 0, 6));
}

#[test]
fn item_through_a_node_box_hits_once() {
    let nodes = vec![
        r(0.0, 0.0, 10.0, 10.0),
        r(100.0, 0.0, 10.0, 10.0),
        r(40.0, -5.0, 20.0, 20.0),
    ];
    let e = item(0, 1, &[(10.0, 5.0), (50.0, 5.0), (100.0, 5.0)]);
    // The two own nodes are ignored; node 2 is hit once even though several points are inside.
    assert_eq!(
        measure_items(std::slice::from_ref(&e), &nodes),
        m(1, 0, 1, 0)
    );
    // Passing within the -2 margin of the border is not a hit.
    let edge = item(0, 1, &[(10.0, 5.0), (41.0, 5.0), (100.0, 5.0)]);
    assert_eq!(measure_items(&[edge], &nodes), m(1, 0, 0, 0));
    // Two items through the same node count twice.
    assert_eq!(measure_items(&[e.clone(), e], &nodes).box_hits, 2);
}

#[test]
fn item_outside_a_node_bbox_does_not_hit() {
    let nodes = vec![
        r(0.0, 0.0, 10.0, 10.0),
        r(100.0, 0.0, 10.0, 10.0),
        r(40.0, 50.0, 20.0, 20.0),
    ];
    let e = item(0, 1, &[(10.0, 5.0), (100.0, 5.0)]);
    assert_eq!(measure_items(&[e], &nodes), m(1, 0, 0, 0));
}

#[test]
fn bends_count_direction_changes_per_axis() {
    let far = far();
    // Straight line: none.
    assert_eq!(
        measure_items(&[item(0, 1, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)])], &far).bends,
        0
    );
    // x: right then left is one bend.
    assert_eq!(
        measure_items(&[item(0, 1, &[(0.0, 0.0), (10.0, 0.0), (0.0, 0.0)])], &far).bends,
        1
    );
    // x and y change on the same step: two bends.
    let e = item(0, 1, &[(0.0, 0.0), (10.0, 10.0), (0.0, 0.0)]);
    assert_eq!(measure_items(&[e], &far).bends, 2);
    // Steps below 0.4 are ignored (no bend, and they do not reset the last direction).
    let e = item(0, 1, &[(0.0, 0.0), (10.0, 0.0), (9.7, 0.0), (20.0, 0.0)]);
    assert_eq!(measure_items(&[e], &far).bends, 0);
    // Exactly 0.4 counts as a direction.
    let e = item(0, 1, &[(0.0, 0.0), (10.0, 0.0), (9.6, 0.0)]);
    assert_eq!(measure_items(&[e], &far).bends, 1);
    // An S shape in y with a pause in x.
    let e = item(0, 1, &[(0.0, 0.0), (0.0, 10.0), (0.0, 0.0), (0.0, 10.0)]);
    assert_eq!(measure_items(&[e], &far).bends, 2);
}

#[test]
fn metrics_combine() {
    let nodes = vec![
        r(-5.0, -5.0, 4.0, 4.0),
        r(200.0, 200.0, 10.0, 10.0),
        r(100.0, 100.0, 10.0, 10.0),
        r(300.0, 300.0, 10.0, 10.0),
    ];
    let es = [
        item(
            0,
            1,
            &[(0.0, 0.0), (100.0, 0.0), (105.0, 105.0), (200.0, 200.0)],
        ),
        item(2, 3, &[(50.0, -20.0), (50.0, 20.0)]),
    ];
    assert_eq!(measure_items(&es, &nodes), m(2, 1, 1, 0));
}
