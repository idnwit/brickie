//! Pointer handling of the stage: pan, brick drag and drag-box selection (`mousemove` and
//! `mouseup` live on the window).

use crate::layout::Rect;
use crate::platform;
use crate::state::*;

/// Pointer handling shared by the stage: `mousemove` / `mouseup` live on the window and act
/// on the level on screen. Registered once per page.
pub fn pointer(cur: &Cur) {
    let cur1 = cur.clone();
    platform::on_mouse("mousemove", move |x, y| {
        let Some(c) = current(&cur1) else { return };
        if let Some(op) = crate::components::editor::pointer_op(&c, x, y) {
            on_move(&c, op, x, y);
        }
    });
    let cur2 = cur.clone();
    platform::on_mouse("mouseup", move |x, y| {
        let Some(c) = current(&cur2) else { return };
        on_up(&c, x, y);
    });
}

/// The pointer moved to client `(x, y)` during `op`: pan, move the dragged brick (past a
/// 3 px threshold) or stretch the selection box.
fn on_move(c: &Cx, op: Op, x: f64, y: f64) {
    match op {
        Op::Pan { sx, sy, vx, vy } => {
            let k = c.view.with(|v| v.k);
            c.view.set(Vw {
                x: vx + x - sx,
                y: vy + y - sy,
                k,
            });
        }
        Op::Drag {
            b,
            sx,
            sy,
            dx0,
            dy0,
        } => {
            let k = c.view.with(|v| v.k);
            let (dx, dy) = ((x - sx) / k, (y - sy) / k);
            if dx.abs() + dy.abs() > 3.0 {
                c.moved.set(true);
            }
            if c.moved.get() {
                c.drag.update(|d| {
                    d.insert(b, (dx0 + dx, dy0 + dy));
                });
            }
        }
        Op::Box { x0, y0, .. } => {
            if let Some((l, t, _, _)) = platform::stage_rect() {
                c.op.set(Some(Op::Box {
                    x0,
                    y0,
                    x1: x - l,
                    y1: y - t,
                }));
            }
        }
    }
}

/// The pointer was released at client `(x, y)`: finish the editor's drop, then a drag-box
/// selects the bricks it touches (when stretched past 4 px); the operation ends.
fn on_up(c: &Cx, x: f64, y: f64) {
    crate::components::editor::release(c, x, y, c.op.get());
    if let Some(Op::Box { x0, y0, x1, y1 }) = c.op.get() {
        let v = c.view.get();
        let to_world = |x: f64, y: f64| ((x - v.x) / v.k, (y - v.y) / v.k);
        let lo = to_world(x0.min(x1), y0.min(y1));
        let hi = to_world(x0.max(x1), y0.max(y1));
        if (x1 - x0).abs() + (y1 - y0).abs() > 4.0 {
            let pos = positions(&c.m.layout, &c.drag.get_clone());
            c.moved.set(true);
            c.select(box_hits(&pos, lo, hi));
        }
    }
    c.op.set(None);
}

/// The selection for a drag-box with world corners `lo` and `hi`: the bricks whose boxes
/// intersect it (touching edges do not count), none / one / a group by their number.
fn box_hits(pos: &[Rect], lo: (f64, f64), hi: (f64, f64)) -> Sel {
    let (ax, ay) = lo;
    let (bx, by) = hi;
    let hit: Vec<usize> = (0..pos.len())
        .filter(|&i| {
            pos[i].x < bx && pos[i].x + pos[i].w > ax && pos[i].y < by && pos[i].y + pos[i].h > ay
        })
        .collect();
    match hit.len() {
        0 => Sel::None,
        1 => Sel::Brick(hit[0]),
        _ => Sel::Group {
            tags: vec![],
            bricks: hit,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            w: 10.0,
            h: 10.0,
        }
    }

    #[test]
    fn box_hits_by_count_and_strict_overlap() {
        let pos = [r(0.0, 0.0), r(20.0, 0.0), r(100.0, 100.0)];
        assert_eq!(box_hits(&pos, (50.0, 50.0), (60.0, 60.0)), Sel::None);
        assert_eq!(box_hits(&pos, (5.0, 5.0), (8.0, 8.0)), Sel::Brick(0));
        assert_eq!(
            box_hits(&pos, (5.0, 5.0), (25.0, 8.0)),
            Sel::Group {
                tags: vec![],
                bricks: vec![0, 1]
            }
        );
        // Sharing only an edge is not a hit (either side); no bricks at all selects none.
        assert_eq!(box_hits(&pos, (10.0, 0.0), (20.0, 10.0)), Sel::None);
        assert_eq!(box_hits(&[], (0.0, 0.0), (1.0, 1.0)), Sel::None);
    }
}
