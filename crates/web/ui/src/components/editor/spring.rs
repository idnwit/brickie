//! Spring-loaded zoom: a drag (a brick, a tile or a lifted brick) that rests on a group brick
//! for [`SPRING_MS`] opens that group's level. A brick dragged on its own level is lifted off
//! it first (`Carry::Brick`): it keeps following the pointer on the new level and can be
//! dropped into a deeper brick, up, or anywhere there (`move_brick`).

use super::aim::clear;
use crate::platform;
use crate::state::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// How long a drag rests on a group before it opens.
pub const SPRING_MS: i32 = 700;

static NEXT: AtomicU32 = AtomicU32::new(1);

/// The group brick a drag aimed at `t` would open, by name.
fn group_of(cx: &Cx, t: Option<&MoveTarget>) -> Option<String> {
    match t {
        Some(MoveTarget::Into(i)) => {
            let b = cx.m.g.bricks.get(*i)?;
            b.file.is_some().then(|| b.name.clone())
        }
        _ => None,
    }
}

/// Arm, keep or drop the spring for a drag aimed at `t` with the pointer at client `(x, y)`.
pub fn arm(cx: &Cx, t: Option<&MoveTarget>, x: f64, y: f64) {
    let ed = cx.ed;
    let level = cx.level().path.clone();
    let Some(name) = group_of(cx, t) else {
        if ed.spring.with_untracked(Option::is_some) {
            ed.spring.set(None);
        }
        return;
    };
    let same = ed.spring.with_untracked(|s| {
        s.as_ref()
            .filter(|s| s.level == level && s.name == name)
            .map(|s| s.id)
    });
    let id = same.unwrap_or_else(|| NEXT.fetch_add(1, Ordering::Relaxed));
    ed.spring.set(Some(Spring {
        id,
        level,
        name,
        x,
        y,
    }));
    if same.is_none() {
        platform::after(SPRING_MS, move || fire(ed, id));
    }
}

/// The timer of hover `id` ran out: open the group if the drag still rests on it.
fn fire(ed: Edit, id: u32) {
    let Some(s) = ed.spring.get_clone_untracked().filter(|s| s.id == id) else {
        return;
    };
    ed.spring.set(None);
    let Some(cx) = current_cx().filter(|c| c.level().path == s.level && c.editing()) else {
        return;
    };
    if let Some(Op::Drag { b, dx0, dy0, .. }) = cx.op.get_untracked() {
        if !cx.moved.get_untracked() || b >= cx.m.g.bricks.len() {
            return;
        }
        // Lift the brick: it goes back to its place here and follows the pointer as a carry.
        let name = cx.m.g.bricks[b].name.clone();
        cx.drag.update(|d| {
            d.insert(b, (dx0, dy0));
        });
        cx.op.set(None);
        cx.moved.set(false);
        let lifted = Carry::Brick {
            level: s.level.clone(),
            name,
        };
        ed.carry.set(Some((lifted, s.x, s.y)));
    } else if ed.carry.with_untracked(Option::is_none) {
        return;
    }
    clear(ed);
    let child = format!("{}/{}", s.level, s.name);
    cx.go(cx.params.with_untracked(|p| p.at_level(&child)));
}
