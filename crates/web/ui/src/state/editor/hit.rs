//! Hit tests of the editor's gestures in canvas (world) space: bricks, tag lanes, the level
//! frame's edge, and where a dragged brick would move.

use super::{MoveTarget, layout_key};
use crate::layout::{Rect, RowOut};
use brickie_web_api::BrickView;
use brickie_web_api::change::LayoutHint;
use std::collections::BTreeMap;

/// How long (ms) a brick must be held over a sibling before release moves it inside.
pub const HOLD_MS: f64 = 400.0;

/// Width (world units) of the band around the frame where a drop moves the brick up.
const EDGE_BAND: f64 = 28.0;

/// Brick boxes moved to their stored canvas positions (level `lv`).
pub fn apply_hints(
    pos: &mut [Rect],
    bricks: &[BrickView],
    hints: &BTreeMap<String, LayoutHint>,
    lv: &str,
) {
    for (r, b) in pos.iter_mut().zip(bricks) {
        if let Some(h) = hints.get(&layout_key(lv, &b.name)).filter(|_| !b.ghost) {
            r.x = h.x;
            r.y = h.y;
        }
    }
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

/// Where a dragged brick, held over `over` since `since` (ms), would go at time `now`: a sibling
/// held long enough, the parent on the frame edge, or nothing.
pub fn move_target(
    bricks: &[BrickView],
    over: Option<usize>,
    hold: Option<(usize, f64)>,
    now: f64,
    up: Option<String>,
) -> Option<MoveTarget> {
    if let Some(t) = over {
        if bricks[t].ghost {
            return Some(MoveTarget::No("outside this level".into()));
        }
        let held = hold.is_some_and(|(h, since)| h == t && now - since >= HOLD_MS);
        return held.then_some(MoveTarget::Into(t));
    }
    up.map(MoveTarget::Up)
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

    #[test]
    fn holding_over_a_sibling_moves_into_it() {
        let bricks = vec![brick("a", false), brick("b", false), brick("x", true)];
        let early = move_target(&bricks, Some(1), Some((1, 0.0)), 100.0, None);
        assert_eq!(early, None);
        let held = move_target(&bricks, Some(1), Some((1, 0.0)), 500.0, None);
        assert_eq!(held, Some(MoveTarget::Into(1)));
        let ghost = move_target(&bricks, Some(2), None, 0.0, None);
        assert!(matches!(ghost, Some(MoveTarget::No(_))));
        let up = move_target(&bricks, None, None, 0.0, Some("root".into()));
        assert_eq!(up, Some(MoveTarget::Up("root".into())));
    }
}
