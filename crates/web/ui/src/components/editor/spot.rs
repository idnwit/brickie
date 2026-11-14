//! Where a planned edge puts its marks ("?" without a reason, "A" when an agent added it): the
//! point along its route nearest the middle that is clear of every box and of the marks already
//! placed, so marks neither sit on brick text nor pile up where edges cross.

use crate::layout::{Pt, Rect, Route};

/// Clearance of a mark from a box border and between two marks (world units).
const BOX_ROOM: f64 = 8.0;
const MARK_ROOM: f64 = 22.0;

/// The mark anchor of route `r` among boxes `p`, avoiding the anchors in `placed` (and adding
/// its own). Falls back to the route middle when no sampled point is clear.
pub fn mark_spot(r: &Route, p: &[Rect], placed: &mut Vec<Pt>) -> Pt {
    let pts = r.sample(6.0);
    let n = pts.len();
    let clear = |q: &Pt| {
        !p.iter().any(|b| b.contains(q, BOX_ROOM))
            && placed
                .iter()
                .all(|o| (o.x - q.x).hypot(o.y - q.y) >= MARK_ROOM)
    };
    // Middle first, then alternately further towards both ends.
    let mid = n / 2;
    let spot = (0..n)
        .map(|k| {
            if k % 2 == 0 {
                mid + k / 2
            } else {
                mid.wrapping_sub(k / 2 + 1)
            }
        })
        .filter(|&i| i < n)
        .map(|i| pts[i])
        .find(clear)
        .unwrap_or_else(|| r.mid());
    placed.push(spot);
    spot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x0: f64, x1: f64) -> Route {
        let at = |x| Pt { x, y: 0.0 };
        Route {
            segs: vec![[
                at(x0),
                at(x0 + (x1 - x0) / 3.0),
                at(x0 + 2.0 * (x1 - x0) / 3.0),
                at(x1),
            ]],
        }
    }

    #[test]
    fn free_middle_is_kept_and_marks_keep_apart_and_off_boxes() {
        let r = line(0.0, 200.0);
        let mut placed = Vec::new();
        let a = mark_spot(&r, &[], &mut placed);
        assert!((a.x - 100.0).abs() < 7.0);
        // The same route again: the second mark moves away from the first.
        let b = mark_spot(&r, &[], &mut placed);
        assert!((a.x - b.x).abs() >= MARK_ROOM);
        // A box over the middle pushes the mark out of it.
        let bx = Rect {
            x: 60.0,
            y: -20.0,
            w: 80.0,
            h: 40.0,
        };
        let c = mark_spot(&r, &[bx], &mut Vec::new());
        assert!(!bx.contains(&c, BOX_ROOM));
    }
}
