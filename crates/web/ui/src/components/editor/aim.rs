//! Aiming a drag: what lies under the pointer (brick body, new-lane zone, breadcrumb or frame
//! edge, tag lane) and what the release would do, kept in the editor's signals for the canvas
//! highlight and the ghost's result label.

use super::drop::world;
use crate::layout::Rect;
use crate::platform;
use crate::state::*;
use brickie_web_api::Params;

/// The level a breadcrumb under the pointer names (an ancestor of the level on screen).
fn crumb_level(x: f64, y: f64) -> Option<String> {
    let href = platform::crumb_at(x, y)?;
    let q = href.split_once('?').map_or("", |(_, q)| q);
    Some(Params::from_query(q).level_path().to_string())
        .map(|l| if l.is_empty() { "root".into() } else { l })
}

/// Tags a rule selector of the level on screen may name (its lanes).
pub fn level_tags(cx: &Cx) -> Vec<String> {
    cx.m.design
        .as_deref()
        .and_then(|c| c.levels.get(&cx.level().path))
        .map(|l| l.tags.clone())
        .unwrap_or_default()
}

fn inside(r: &Rect, (x, y): (f64, f64)) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

/// What is under client point `(x, y)`, the dragged brick `skip` aside.
fn under(cx: &Cx, x: f64, y: f64, skip: Option<usize>) -> Under {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone_untracked());
    let Some(at) = world(cx, x, y) else {
        let up = crumb_level(x, y).filter(|l| *l != cx.level().path);
        return Under {
            up,
            ..Default::default()
        };
    };
    let frame = frame_of(&cx.m.g.bricks, &pos, skip);
    let rows = &cx.m.layout.rows;
    let edge = frame.is_some_and(|f| on_frame_edge(&f, at));
    let up = crumb_level(x, y)
        .filter(|l| *l != cx.level().path)
        .or_else(|| edge.then(|| parent_level(&cx.level().path)).flatten());
    let in_frame = frame.is_some_and(|f| inside(&f, at));
    Under {
        brick: hit_any(&pos, at, skip),
        new_lane: new_lane_zone(rows, &cx.m.g.bricks, &pos, skip).is_some_and(|z| inside(&z, at)),
        up,
        lane: in_frame
            .then(|| lane_hit(rows, &level_tags(cx), at.1))
            .flatten(),
    }
}

/// Aim `what` at client `(x, y)`: set the target, the brick under it and the ghost label.
pub fn aim(cx: &Cx, what: Carried, x: f64, y: f64) {
    let ed = cx.ed;
    let skip = match what {
        Carried::Brick(b) => Some(b),
        Carried::Tile | Carried::Lifted => None,
    };
    let u = under(cx, x, y, skip);
    let t = drop_target(&cx.m.g.bricks, what, &u);
    let label = drop_label(&cx.m.g.bricks, what, t.as_ref());
    if ed.over.get_untracked() != u.brick {
        ed.over.set(u.brick);
    }
    super::spring::arm(cx, t.as_ref(), x, y);
    if ed.target.with_untracked(|o| *o != t) {
        ed.target.set(t);
    }
    if ed.aim_label.with_untracked(|l| *l != label) {
        ed.aim_label.set(label);
    }
}

/// How a carry aims on the level on screen: a lifted brick back on its own level is a brick
/// drag there.
pub fn carried(cx: &Cx, c: &Carry) -> Carried {
    match c {
        Carry::Tile(_) => Carried::Tile,
        Carry::Brick { level, name } if *level == cx.level().path => {
            cx.brick(name).map_or(Carried::Lifted, Carried::Brick)
        }
        Carry::Brick { .. } => Carried::Lifted,
    }
}

/// The drag ended: clear the aim. Only what is set changes, so a plain click does not
/// re-render the layer under the pointer (its click would be lost).
pub fn clear(ed: Edit) {
    if ed.target.with_untracked(Option::is_some) {
        ed.target.set(None);
    }
    if ed.over.get_untracked().is_some() {
        ed.over.set(None);
    }
    if ed.aim_label.with_untracked(|l| !l.is_empty()) {
        ed.aim_label.set(String::new());
    }
    if ed.spring.with_untracked(Option::is_some) {
        ed.spring.set(None);
    }
}
