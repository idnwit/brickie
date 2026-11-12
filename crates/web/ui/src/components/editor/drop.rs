//! Pointer hooks of edit mode, called on pointer release: a click on the stage places the armed
//! palette kind; releasing the pointer drops a carried kind, a picked link or a dragged brick
//! (onto another brick: move inside; elsewhere: store its position).

use super::send;
use crate::platform;
use crate::state::*;
use brickie_web_api::change::LayoutHint;
use brickie_web_api::{ChangeLayout, ChangeLayoutRequest};

/// Client point -> canvas (world) point, when inside the stage.
fn world(cx: &Cx, x: f64, y: f64) -> Option<(f64, f64)> {
    let (l, t, w, h) = platform::stage_rect()?;
    let (sx, sy) = (x - l, y - t);
    if sx < 0.0 || sy < 0.0 || sx > w || sy > h {
        return None;
    }
    let v = cx.view.get();
    Some(((sx - v.x) / v.k, (sy - v.y) / v.k))
}

/// Place the armed kind at client `(x, y)`. True when a kind was armed.
fn place_click(cx: &Cx, x: f64, y: f64) -> bool {
    let ed = cx.ed;
    let Some(kind) = ed.tool.get().filter(|_| cx.editing()) else {
        return false;
    };
    if let Some((wx, wy)) = world(cx, x, y) {
        ed.draft.set(Some(Draft { kind, x: wx, y: wy }));
        ed.tool.set(None);
        platform::next_frame(|| platform::focus("draft-name"));
    }
    true
}

/// The pointer went up at client `(x, y)` during `op`.
pub fn release(cx: &Cx, x: f64, y: f64, op: Option<Op>) {
    if !cx.editing() {
        return;
    }
    let ed = cx.ed;
    if ed.carry.get() {
        ed.carry.set(false);
        if world(cx, x, y).is_some() {
            place_click(cx, x, y);
        }
        return;
    }
    // A click (no pan) on the stage background places the armed kind.
    if let Some(Op::Pan { sx, sy, .. }) = op
        && (x - sx).abs() + (y - sy).abs() < 4.0
        && place_click(cx, x, y)
    {
        return;
    }
    let pos = positions(&cx.m.layout, &cx.drag.get_clone());
    let at = world(cx, x, y);
    if let Some(pick) = ed.link.get_clone() {
        let from = cx.brick(&pick.from);
        if let Some(t) = at.and_then(|p| hit(&pos, &cx.m.g.bricks, p, from)) {
            let to = cx.m.g.bricks[t].name.clone();
            ed.pending.set(Some(Pending::Rule { pick, to }));
            platform::next_frame(|| platform::focus("rule-reason"));
        }
        return;
    }
    let Some(Op::Drag { b, .. }) = op else { return };
    if !cx.moved.get() || cx.m.g.bricks[b].ghost {
        return;
    }
    let name = cx.m.g.bricks[b].name.clone();
    if let Some(t) = at.and_then(|p| hit(&pos, &cx.m.g.bricks, p, Some(b))) {
        let into = cx.m.g.bricks[t].name.clone();
        ed.pending.set(Some(Pending::Move { from: name, into }));
        return;
    }
    let req = ChangeLayoutRequest {
        brick: layout_key(&cx.level().path, &name),
        hint: Some(LayoutHint {
            x: pos[b].x,
            y: pos[b].y,
        }),
    };
    send::<ChangeLayout>(ed, req, |_| {});
}
