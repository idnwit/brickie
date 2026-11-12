//! Pointer handling of the stage: pan, brick drag and drag-box selection (`mousemove` and
//! `mouseup` live on the window).

use crate::platform;
use crate::state::*;

/// Pointer handling shared by the stage: `mousemove` / `mouseup` live on the window and act
/// on the level on screen. Registered once per page.
pub fn pointer(cur: &Cur) {
    let cur1 = cur.clone();
    platform::on_mouse("mousemove", move |x, y| {
        let Some(c) = current(&cur1) else { return };
        let Some(op) = crate::components::editor::pointer_op(&c, x, y) else {
            return;
        };
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
    });
    let cur2 = cur.clone();
    platform::on_mouse("mouseup", move |x, y| {
        let Some(c) = current(&cur2) else { return };
        crate::components::editor::release(&c, x, y, c.op.get());
        if let Some(Op::Box { x0, y0, x1, y1 }) = c.op.get() {
            // Drag-box selection: bricks whose boxes intersect the rectangle.
            let v = c.view.get();
            let to_world = |x: f64, y: f64| ((x - v.x) / v.k, (y - v.y) / v.k);
            let (ax, ay) = to_world(x0.min(x1), y0.min(y1));
            let (bx, by) = to_world(x0.max(x1), y0.max(y1));
            if (x1 - x0).abs() + (y1 - y0).abs() > 4.0 {
                let pos = positions(&c.m.layout, &c.drag.get_clone());
                let hit: Vec<usize> = (0..pos.len())
                    .filter(|&i| {
                        pos[i].x < bx
                            && pos[i].x + pos[i].w > ax
                            && pos[i].y < by
                            && pos[i].y + pos[i].h > ay
                    })
                    .collect();
                c.moved.set(true);
                let sel = match hit.len() {
                    0 => Sel::None,
                    1 => Sel::Brick(hit[0]),
                    _ => Sel::Group {
                        tags: vec![],
                        bricks: hit,
                    },
                };
                c.select(sel);
            }
        }
        c.op.set(None);
    });
}
