//! Hit tests of the editor's gestures in canvas (world) space: bricks, tag lanes, the level
//! frame's edge, and where a dragged brick would move.

use super::layout_key;
use crate::layout::{Rect, RowOut};
use brickie_web_api::BrickView;
use brickie_web_api::LayoutHint;
use std::collections::BTreeMap;

/// Width (world units) of the band around the frame where a drop moves the brick up.
const EDGE_BAND: f64 = 28.0;

/// Gap (world units) between a brick placed in free space and its neighbours.
const FREE_GAP: f64 = 24.0;

/// Brick boxes moved to their stored canvas positions (level `lv`). A brick without a stored
/// position (a new one) that would overlap a box is moved into free space, right of the boxes.
pub fn apply_hints(
    pos: &mut [Rect],
    bricks: &[BrickView],
    hints: &BTreeMap<String, LayoutHint>,
    lv: &str,
) {
    let mut placed = vec![false; pos.len()];
    for (i, (r, b)) in pos.iter_mut().zip(bricks).enumerate() {
        if let Some(h) = hints.get(&layout_key(lv, &b.name)).filter(|_| !b.ghost) {
            r.x = h.x;
            r.y = h.y;
            placed[i] = true;
        }
    }
    if placed.iter().any(|p| *p) {
        place_free(pos, bricks, &placed);
    }
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Move each brick not `placed` that overlaps another box to the right of every box.
pub fn place_free(pos: &mut [Rect], bricks: &[BrickView], placed: &[bool]) {
    let n = pos.len().min(bricks.len());
    let mut right = (0..n)
        .filter(|&i| !bricks[i].ghost)
        .map(|i| pos[i].x + pos[i].w)
        .fold(f64::MIN, f64::max);
    for i in (0..n).filter(|&i| !placed[i] && !bricks[i].ghost) {
        let hit = (0..n).any(|j| j != i && !bricks[j].ghost && overlaps(&pos[i], &pos[j]));
        if hit {
            pos[i].x = right + FREE_GAP;
            right = pos[i].x + pos[i].w;
        }
    }
}

/// `hints` without the bricks directly on level `lv` (auto layout of that level).
pub fn without_level_hints(
    hints: &BTreeMap<String, LayoutHint>,
    lv: &str,
) -> BTreeMap<String, LayoutHint> {
    (hints.iter())
        .filter(|(k, _)| k.rsplit_once('/').is_none_or(|(p, _)| p != lv))
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

fn contains(r: &Rect, (x, y): (f64, f64)) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

/// The brick (not a faded neighbour, not `skip`) whose box holds the world point.
pub fn hit(
    pos: &[Rect],
    bricks: &[BrickView],
    at: (f64, f64),
    skip: Option<usize>,
) -> Option<usize> {
    (0..pos.len().min(bricks.len()))
        .find(|&i| Some(i) != skip && !bricks[i].ghost && contains(&pos[i], at))
}

/// Any brick but `skip` under the point, faded neighbours included.
pub fn hit_any(pos: &[Rect], at: (f64, f64), skip: Option<usize>) -> Option<usize> {
    (0..pos.len()).find(|&i| Some(i) != skip && contains(&pos[i], at))
}

/// The tag of the lane (row) at world height `y`, among the level's `tags`.
pub fn lane_at(rows: &[RowOut], tags: &[String], y: f64) -> Option<String> {
    rows.iter()
        .find(|r| y >= r.y && y <= r.y + r.h && tags.contains(&r.label))
        .map(|r| r.label.clone())
}

/// Whether the point is on the band around the frame (just outside or on its border).
pub fn on_frame_edge(frame: &Rect, (x, y): (f64, f64)) -> bool {
    let outer = Rect {
        x: frame.x - EDGE_BAND,
        y: frame.y - EDGE_BAND,
        w: frame.w + 2.0 * EDGE_BAND,
        h: frame.h + 2.0 * EDGE_BAND,
    };
    let inner = Rect {
        x: frame.x + 6.0,
        y: frame.y + 6.0,
        w: (frame.w - 12.0).max(0.0),
        h: (frame.h - 12.0).max(0.0),
    };
    contains(&outer, (x, y)) && !contains(&inner, (x, y))
}

/// The parent of level `lv` (`root/web` -> `root`), `None` at the root.
pub fn parent_level(lv: &str) -> Option<String> {
    lv.rsplit_once('/').map(|(up, _)| up.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brick(name: &str, ghost: bool) -> BrickView {
        BrickView {
            name: name.into(),
            ghost,
            ..Default::default()
        }
    }

    fn rect(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            w: 100.0,
            h: 50.0,
        }
    }

    #[test]
    fn hints_move_boxes_and_hits_skip_ghosts() {
        let bricks = vec![brick("a", false), brick("out", true)];
        let mut pos = vec![rect(0.0, 0.0), rect(200.0, 0.0)];
        let hints = BTreeMap::from([
            ("root/a".to_string(), LayoutHint { x: 300.0, y: 40.0 }),
            ("root/out".to_string(), LayoutHint { x: 9.0, y: 9.0 }),
        ]);
        apply_hints(&mut pos, &bricks, &hints, "root");
        assert_eq!((pos[0].x, pos[0].y, pos[1].x), (300.0, 40.0, 200.0));
        assert_eq!(hit(&pos, &bricks, (310.0, 50.0), None), Some(0));
        assert_eq!(hit(&pos, &bricks, (310.0, 50.0), Some(0)), None);
        assert_eq!(hit(&pos, &bricks, (210.0, 10.0), None), None);
        assert_eq!(hit_any(&pos, (210.0, 10.0), None), Some(1));
        assert_eq!(hit_any(&pos, (210.0, 10.0), Some(1)), None);
    }

    #[test]
    fn a_new_brick_is_placed_clear_of_the_stored_boxes() {
        let bricks = vec![brick("a", false), brick("b", false), brick("new", false)];
        let mut pos = vec![rect(0.0, 0.0), rect(200.0, 0.0), rect(0.0, 0.0)];
        let hints = BTreeMap::from([
            ("root/a".to_string(), LayoutHint { x: 0.0, y: 0.0 }),
            ("root/b".to_string(), LayoutHint { x: 150.0, y: 0.0 }),
        ]);
        apply_hints(&mut pos, &bricks, &hints, "root");
        assert_eq!((pos[0].x, pos[1].x), (0.0, 150.0));
        assert_eq!(pos[2].x, 250.0 + FREE_GAP);
        assert!(!overlaps(&pos[2], &pos[0]) && !overlaps(&pos[2], &pos[1]));
    }

    #[test]
    fn auto_layout_forgets_only_the_level_hints() {
        let h = LayoutHint { x: 1.0, y: 1.0 };
        let hints = BTreeMap::from([("root/a".to_string(), h), ("root/web/x".to_string(), h)]);
        let left = without_level_hints(&hints, "root");
        assert_eq!(left.keys().collect::<Vec<_>>(), vec!["root/web/x"]);
        assert_eq!(without_level_hints(&hints, "root/web").len(), 1);
    }

    #[test]
    fn lanes_and_frame_edge() {
        let rows = vec![RowOut {
            y: 0.0,
            h: 80.0,
            label: "io".into(),
            ids: vec![],
        }];
        assert_eq!(lane_at(&rows, &["io".into()], 40.0).as_deref(), Some("io"));
        assert_eq!(lane_at(&rows, &[], 40.0), None);
        let f = rect(0.0, 0.0);
        assert!(on_frame_edge(&f, (-10.0, 20.0)));
        assert!(!on_frame_edge(&f, (50.0, 25.0)));
        assert!(!on_frame_edge(&f, (-100.0, 20.0)));
        assert_eq!(parent_level("root/web").as_deref(), Some("root"));
        assert_eq!(parent_level("root"), None);
    }
}
