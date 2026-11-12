//! The stage's own mouse handlers (pan or box start, wheel zoom, background click) and the
//! pure decisions behind them.

use crate::platform;
use crate::state::*;

/// The view after a wheel turn of `delta_y` at stage point `(mx, my)`: zoom in `[0.15, 3]`,
/// the point under the pointer stays put.
fn zoom_at(v: Vw, mx: f64, my: f64, delta_y: f64) -> Vw {
    let k = (v.k * (-delta_y * 0.0015).exp()).clamp(0.15, 3.0);
    Vw {
        x: mx - (mx - v.x) * (k / v.k),
        y: my - (my - v.y) * (k / v.k),
        k,
    }
}

/// CSS of the drag-box overlay for the operation in progress (hidden unless a box).
pub fn box_style(op: Option<Op>) -> String {
    match op {
        Some(Op::Box { x0, y0, x1, y1 }) => format!(
            "left:{}px;top:{}px;width:{}px;height:{}px",
            x0.min(x1),
            y0.min(y1),
            (x1 - x0).abs(),
            (y1 - y0).abs()
        ),
        _ => "display:none".to_string(),
    }
}

/// `mousedown`: start a pan, or a drag-box with shift (left button only).
pub fn down(cx: &Cx) -> impl Fn(Click) + 'static {
    let c = cx.clone();
    move |e: Click| {
        if e.button() != 0 {
            return;
        }
        let Some((l, t, _, _)) = platform::stage_rect() else {
            return;
        };
        c.moved.set(false);
        if e.shift_key() {
            let (x, y) = (e.client_x() as f64 - l, e.client_y() as f64 - t);
            c.op.set(Some(Op::Box {
                x0: x,
                y0: y,
                x1: x,
                y1: y,
            }));
        } else {
            let v = c.view.get();
            c.op.set(Some(Op::Pan {
                sx: e.client_x() as f64,
                sy: e.client_y() as f64,
                vx: v.x,
                vy: v.y,
            }));
        }
    }
}

/// `wheel`: zoom at the pointer (the first one also records the stage size).
pub fn wheel(cx: &Cx) -> impl Fn(web_sys::WheelEvent) + 'static {
    let c = cx.clone();
    move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let Some((l, t, sw, sh)) = platform::stage_rect() else {
            return;
        };
        if c.stage.get().is_none() {
            c.stage.set(Some((sw, sh)));
        }
        let (mx, my) = (e.client_x() as f64 - l, e.client_y() as f64 - t);
        c.view.set(zoom_at(c.view.get(), mx, my, e.delta_y()));
    }
}

/// `click` on the background: ends a drag (swallowed), else clears an open item or selection.
pub fn bg_click(cx: &Cx) -> impl Fn(Click) + 'static {
    let c = cx.clone();
    move |_: Click| {
        if c.moved.get() {
            c.moved.set(false);
            return;
        }
        let p = c.params.get_clone();
        let item_or_sel =
            !p.item.is_empty() || !p.brick.is_empty() || !p.group.is_empty() || !p.edge.is_empty();
        if item_or_sel {
            c.select(Sel::None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_point_under_the_pointer_and_clamps() {
        let v = Vw {
            x: 10.0,
            y: 20.0,
            k: 1.0,
        };
        // No wheel turn: unchanged.
        assert_eq!(zoom_at(v, 50.0, 60.0, 0.0), v);
        // Zoom in at (50, 60): that stage point maps to the same world point.
        let z = zoom_at(v, 50.0, 60.0, -100.0);
        assert!(z.k > 1.0);
        let world = |v: Vw| ((50.0 - v.x) / v.k, (60.0 - v.y) / v.k);
        let (a, b) = (world(v), world(z));
        assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        assert_eq!(zoom_at(v, 0.0, 0.0, -1e6).k, 3.0);
        assert_eq!(zoom_at(v, 0.0, 0.0, 1e6).k, 0.15);
    }

    #[test]
    fn box_style_normalises_corners_and_hides_other_ops() {
        let op = Some(Op::Box {
            x0: 30.0,
            y0: 5.0,
            x1: 10.0,
            y1: 25.0,
        });
        assert_eq!(box_style(op), "left:10px;top:5px;width:20px;height:20px");
        assert_eq!(box_style(None), "display:none");
    }
}
