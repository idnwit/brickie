//! Pointer hooks of edit mode. On move: a carried brick tile follows the pointer, a port drag
//! updates its rubber band, a dragged brick or tile aims (see `aim`), and hovering shows a
//! brick's ports. On release: a tile becomes an inline-named draft where it was aimed (here, in
//! a lane, inside a brick, in a new lane), the link becomes a rule, a brick moves into a brick
//! or up, changes lane, or just stores its position.

use super::aim::{aim, carried, clear};
use super::{connect, op, send, write};
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
    if let Some((k, _, _)) = ed.carry.get_clone() {
        let what = carried(cx, &k);
        ed.carry.set(Some((k, x, y)));
        aim(cx, what, x, y);
        return true;
    }
    if ed.connect.with(Option::is_some) {
        if let Some(p) = world(cx, x, y) {
            connect::moved(cx, p);
        }
        return true;
    }
    if let (Some(Op::Drag { b, .. }), true) = (cx.op.get(), cx.moved.get()) {
        aim(cx, Carried::Brick(b), x, y);
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

/// Open the inline name box of a new `kind` brick at client `(x, y)`, going to `dest`.
fn draft_at(cx: &Cx, kind: brickie_web_api::change::BrickKind, x: f64, y: f64, dest: Dest) {
    if let Some((wx, wy)) = world(cx, x, y) {
        cx.ed.draft.set(Some(Draft {
            kind,
            x: wx,
            y: wy,
            dest,
        }));
        cx.ed.brick_tool.set(None);
        // The box renders synchronously: focus it now, and again once painted.
        platform::focus("inline-name");
        platform::next_frame(|| platform::focus("inline-name"));
    }
}

/// A tile released where it was aimed.
fn drop_tile(cx: &Cx, kind: brickie_web_api::change::BrickKind, x: f64, y: f64) {
    let target = cx.ed.target.get_clone();
    clear(cx.ed);
    let dest = match target {
        Some(MoveTarget::Into(t)) => Dest::Into(cx.m.g.bricks[t].name.clone()),
        Some(MoveTarget::Lane(l)) => Dest::Here(l.into_iter().collect()),
        Some(MoveTarget::NewLane) => Dest::NewLane,
        Some(MoveTarget::No(why)) => return cx.ed.msg.set(format!("Cannot add there: {why}")),
        Some(MoveTarget::Up(_)) | None => Dest::Here(vec![]),
    };
    draft_at(cx, kind, x, y, dest);
}

/// Move brick `name` of level `from` into the brick at path `into` (`place` words the toast).
pub(super) fn move_brick(cx: &Cx, from: &str, name: String, into: Vec<String>, place: String) {
    let req = ChangeMoveRequest {
        from: brick_path(from, &name),
        into,
        note: None,
    };
    let toast =
        format!("Moved {name} {place} \u{00B7} Ctrl+Z undoes; code is not moved on publish");
    write::<ChangeMove, _>(cx, req, toast, |_, _| {});
}

/// Store brick `b`'s position (`keep`: where it was dropped; else drop the stored one, so the
/// layout places it in its new lane), then `then`.
fn store_position(cx: &Cx, b: usize, keep: bool, then: impl FnOnce() + 'static) {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone());
    let req = ChangeLayoutRequest {
        brick: layout_key(&cx.level().path, &cx.m.g.bricks[b].name),
        hint: keep.then_some(LayoutHint {
            x: pos[b].x,
            y: pos[b].y,
        }),
    };
    send::<ChangeLayout>(cx.ed, req, move |_| then());
}

/// The end of a brick drag: into a brick, up, to another lane, refused, or repositioned.
fn drop_brick(cx: &Cx, b: usize, back: (f64, f64)) {
    let ed = cx.ed;
    let target = ed.target.get_clone();
    clear(ed);
    let brick = &cx.m.g.bricks[b];
    if brick.ghost {
        return;
    }
    let name = brick.name.clone();
    match target {
        Some(MoveTarget::Into(t)) => {
            let into = &cx.m.g.bricks[t].name;
            let place = format!("into {into}");
            let lv = &cx.level().path;
            move_brick(cx, lv, name, brick_path(lv, into), place);
        }
        Some(MoveTarget::Up(level)) => {
            let place = format!("up to {level}");
            move_brick(cx, &cx.level().path, name, level_brick_path(&level), place);
        }
        Some(MoveTarget::Lane(lane)) => {
            let file = cx.level_file().unwrap_or_default();
            let o = retag_op(&file, brick, lane.as_deref());
            let c = cx.clone();
            store_position(cx, b, false, move || o.into_iter().for_each(|o| op(&c, o)));
        }
        Some(MoveTarget::NewLane) => {
            let p = positions(&cx.m.layout, &cx.drag.get_clone())[b];
            ed.lane_ask.set(Some((name, p.x, p.y)));
            platform::next_frame(|| platform::focus("inline-name"));
        }
        Some(MoveTarget::No(why)) => {
            cx.drag.update(|d| {
                d.insert(b, back);
            });
            ed.msg.set(format!("Cannot move {name} there: {why}"));
        }
        None => store_position(cx, b, true, || {}),
    }
}

/// A lifted brick released: on its own level like a brick drag there; elsewhere into the
/// brick under it, up, or onto the level on screen.
fn drop_lifted(cx: &Cx, from: &str, name: String) {
    let here = cx.level().path.clone();
    if let Some(b) = cx.brick(&name).filter(|_| here == *from) {
        let back = cx.drag.with(|d| d.get(&b).copied()).unwrap_or_default();
        return drop_brick(cx, b, back);
    }
    let target = cx.ed.target.get_clone();
    clear(cx.ed);
    let (into, place) = match target {
        Some(MoveTarget::Into(t)) => {
            let into = cx.m.g.bricks[t].name.clone();
            (brick_path(&here, &into), format!("into {into}"))
        }
        Some(MoveTarget::Up(l)) => (level_brick_path(&l), format!("up to {l}")),
        Some(MoveTarget::No(why)) => {
            return cx.ed.msg.set(format!("Cannot move {name} there: {why}"));
        }
        _ => (level_brick_path(&here), format!("to {here}")),
    };
    move_brick(cx, from, name, into, place);
}

/// Esc during a brick drag: put it back and forget the drag. True when there was one.
pub fn cancel_drag(cx: &Cx) -> bool {
    let Some(Op::Drag { b, dx0, dy0, .. }) = cx.op.get() else {
        return false;
    };
    cx.drag.update(|d| {
        d.insert(b, (dx0, dy0));
    });
    cx.op.set(None);
    cx.moved.set(false);
    clear(cx.ed);
    true
}

/// The pointer went up at client `(x, y)` during `op`.
pub fn release(cx: &Cx, x: f64, y: f64, op: Option<Op>) {
    if !cx.editing() {
        return;
    }
    let ed = cx.ed;
    match ed.carry.get_clone() {
        Some((Carry::Tile(kind), _, _)) => {
            ed.carry.set(None);
            return drop_tile(cx, kind, x, y);
        }
        Some((Carry::Brick { level, name }, _, _)) => {
            ed.carry.set(None);
            return drop_lifted(cx, &level, name);
        }
        None => {}
    }
    if ed.connect.with(Option::is_some) {
        connect::finish(cx, world(cx, x, y));
        return;
    }
    match op {
        Some(Op::Pan { sx, sy, .. }) if (x - sx).abs() + (y - sy).abs() < 4.0 => {
            if let Some(kind) = ed.brick_tool.get() {
                // A click with the armed tile: into the brick or lane under it, like a drop.
                super::aim::aim(cx, Carried::Tile, x, y);
                drop_tile(cx, kind, x, y);
            }
        }
        Some(Op::Drag { b, dx0, dy0, .. }) if cx.moved.get() => drop_brick(cx, b, (dx0, dy0)),
        _ => clear(ed),
    }
}
