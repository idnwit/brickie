//! Pointer handling of the stage: pan and brick drag (`mousemove` and
//! `mouseup` live on the window).

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
/// 3 px threshold).
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
    }
}

/// The pointer was released at client `(x, y)`: finish the editor's drop; the operation ends.
fn on_up(c: &Cx, x: f64, y: f64) {
    crate::components::editor::release(c, x, y, c.op.get());
    c.op.set(None);
}
