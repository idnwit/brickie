//! Pointer hooks of edit mode. On move: a carried brick tile follows the pointer, a port drag
//! updates its rubber band, a dragged brick finds where it would go (held over a sibling: move
//! into; on the frame edge or a breadcrumb: move up), and hovering shows a brick's ports. On
//! release: the tile becomes an inline-named draft, the link becomes a rule, the brick moves
//! (or its position is stored).

use super::{connect, send, write};
use crate::components::canvas::frame_rect;
use crate::platform;
use crate::state::*;
use brickie_web_api::change::LayoutHint;
use brickie_web_api::*;

/// Client point -> canvas (world) point, when inside the stage.
pub fn world(cx: &Cx, x: f64, y: f64) -> Option<(f64, f64)> {
    let (l, t, w, h) = platform::stage_rect()?;
    let (sx, sy) = (x - l, y - t);
    if sx < 0.0 || sy < 0.0 || sx > w || sy > h {
        return None;
    }
    let v = cx.view.get();
    Some(((sx - v.x) / v.k, (sy - v.y) / v.k))
}

/// The level a breadcrumb under the pointer names (an ancestor of the level on screen).
fn crumb_level(x: f64, y: f64) -> Option<String> {
    let href = platform::crumb_at(x, y)?;
    let q = href.split_once('?').map_or("", |(_, q)| q);
    Some(Params::from_query(q).level_path().to_string())
        .map(|l| if l.is_empty() { "root".into() } else { l })
}

/// After the hold time, a brick still held over sibling `t` targets it (move into).
fn arm_hold(cx: &Cx, t: usize) {
    let c = cx.clone();
    platform::after(HOLD_MS as i32 + 20, move || {
        // The level may have been re-rendered meanwhile (its signals gone).
        let held = c.op.is_alive()
            && c.ed.hold.get().map(|h| h.0) == Some(t)
            && matches!(c.op.get(), Some(Op::Drag { .. }));
        if held && !c.m.g.bricks[t].ghost {
            c.ed.target.set(Some(MoveTarget::Into(t)));
        }
    });
}

/// Where the dragged brick `b` would go with the pointer at client `(x, y)`.
fn aim(cx: &Cx, b: usize, x: f64, y: f64) {
    let ed = cx.ed;
    let pos = positions(&cx.m.layout, &cx.drag.get_clone_untracked());
    let at = world(cx, x, y);
    let over = at.and_then(|p| hit_any(&pos, p, Some(b)));
    if over != ed.hold.get().map(|h| h.0) {
        ed.hold.set(over.map(|t| (t, platform::now())));
        if let Some(t) = over {
            arm_hold(cx, t);
        }
    }
    let edge = at
        .zip(frame_rect(&cx.m.g, &pos))
        .is_some_and(|(p, f)| on_frame_edge(&f, p));
    let up = crumb_level(x, y)
        .filter(|l| *l != cx.level().path)
        .or_else(|| edge.then(|| parent_level(&cx.level().path)).flatten());
    ed.target.set(move_target(
        &cx.m.g.bricks,
        over,
        ed.hold.get(),
        platform::now(),
        up,
    ));
}

/// The pointer moved to client `(x, y)`: the canvas operation to go on with, `None` when the
/// editor consumed the move (or there is none).
pub fn pointer_op(cx: &Cx, x: f64, y: f64) -> Option<Op> {
    (!pointer_move(cx, x, y)).then(|| cx.op.get()).flatten()
}

/// True when the editor consumed the move.
fn pointer_move(cx: &Cx, x: f64, y: f64) -> bool {
    if !cx.editing() {
        return false;
    }
    let ed = cx.ed;
    if let Some((k, _, _)) = ed.carry.get() {
        ed.carry.set(Some((k, x, y)));
        return true;
    }
    if ed.connect.with(Option::is_some) {
        if let Some(p) = world(cx, x, y) {
            connect::moved(cx, p);
        }
        return true;
    }
    if let (Some(Op::Drag { b, .. }), true) = (cx.op.get(), cx.moved.get()) {
        aim(cx, b, x, y);
        return false;
    }
    if cx.op.get().is_none() {
        hover(cx, x, y);
    }
    false
}

/// Ports show on the brick under the pointer (its box plus the port strip at its right).
fn hover(cx: &Cx, x: f64, y: f64) {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone_untracked());
    let hot = world(cx, x, y).and_then(|(wx, wy)| {
        (0..pos.len().min(cx.m.g.bricks.len())).find(|&i| {
            let r = &pos[i];
            !cx.m.g.bricks[i].ghost
                && wx >= r.x
                && wx <= r.x + r.w + 30.0
                && wy >= r.y - 4.0
                && wy <= r.y + r.h + 4.0
        })
    });
    if cx.ed.hot.get() != hot {
        cx.ed.hot.set(hot);
    }
}

fn draft_at(cx: &Cx, kind: brickie_web_api::change::BrickKind, x: f64, y: f64) {
    if let Some((wx, wy)) = world(cx, x, y) {
        cx.ed.draft.set(Some(Draft { kind, x: wx, y: wy }));
        cx.ed.brick_tool.set(None);
        // The box renders synchronously: focus it now, and again once painted.
        platform::focus("inline-name");
        platform::next_frame(|| platform::focus("inline-name"));
    }
}

fn move_brick(cx: &Cx, name: String, into: Vec<String>) {
    let req = ChangeMoveRequest {
        from: brick_path(&cx.level().path, &name),
        into,
        note: None,
    };
    let toast = format!("Moved {name}; code is not moved on publish (manual step)");
    write::<ChangeMove, _>(cx, req, toast, |_, _| {});
}

fn store_position(cx: &Cx, b: usize) {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone());
    let req = ChangeLayoutRequest {
        brick: layout_key(&cx.level().path, &cx.m.g.bricks[b].name),
        hint: Some(LayoutHint {
            x: pos[b].x,
            y: pos[b].y,
        }),
    };
    send::<ChangeLayout>(cx.ed, req, |_| {});
}

/// The end of a brick drag: move it, refuse, or store its position.
fn drop_brick(cx: &Cx, b: usize) {
    let ed = cx.ed;
    let target = ed.target.get_clone();
    ed.target.set(None);
    ed.hold.set(None);
    if cx.m.g.bricks[b].ghost {
        return;
    }
    let name = cx.m.g.bricks[b].name.clone();
    match target {
        Some(MoveTarget::Into(t)) => {
            let into = brick_path(&cx.level().path, &cx.m.g.bricks[t].name);
            move_brick(cx, name, into);
        }
        Some(MoveTarget::Up(level)) => move_brick(cx, name, level_brick_path(&level)),
        Some(MoveTarget::No(why)) => ed.msg.set(format!("Cannot move {name} there: {why}")),
        None => store_position(cx, b),
    }
}

/// The pointer went up at client `(x, y)` during `op`.
pub fn release(cx: &Cx, x: f64, y: f64, op: Option<Op>) {
    if !cx.editing() {
        return;
    }
    let ed = cx.ed;
    if let Some((kind, _, _)) = ed.carry.get() {
        ed.carry.set(None);
        draft_at(cx, kind, x, y);
        return;
    }
    if ed.connect.with(Option::is_some) {
        connect::finish(cx, world(cx, x, y));
        return;
    }
    match op {
        Some(Op::Pan { sx, sy, .. }) if (x - sx).abs() + (y - sy).abs() < 4.0 => {
            if let Some(kind) = ed.brick_tool.get() {
                draft_at(cx, kind, x, y);
            }
        }
        Some(Op::Drag { b, .. }) if cx.moved.get() => drop_brick(cx, b),
        _ => {}
    }
}
