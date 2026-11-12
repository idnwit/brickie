//! Cut and paste to move a brick across levels: `Ctrl+X` marks the selected brick, `Ctrl+V` on
//! another level moves it there (into the selected brick when one is selected), one
//! `move_brick`. Esc forgets the cut brick.

use super::drop::move_brick;
use super::toast;
use crate::state::*;

/// Mark the selected brick to move (`Ctrl+X`).
pub fn cut(cx: &Cx) {
    let Sel::Brick(b) = cx.eval.with(|ev| ev.sel.clone()) else {
        return cx.ed.msg.set("Select a brick to cut.".into());
    };
    let brick = &cx.m.g.bricks[b];
    if brick.ghost {
        return cx
            .ed
            .msg
            .set("A faded neighbour is outside this level: open its level to cut it.".into());
    }
    cx.ed.clip.set(Some(Clip {
        level: cx.level().path.clone(),
        name: brick.name.clone(),
    }));
    cx.ed.msg.set(String::new());
    let text = format!(
        "Cut {}: open a level (or select a brick there) and press Ctrl+V \u{00B7} Esc cancels",
        brick.name
    );
    toast(cx.ed, text);
}

/// Move the cut brick here, or into the selected brick (`Ctrl+V`).
pub fn paste(cx: &Cx) {
    let Some(c) = cx.ed.clip.get_clone() else {
        return cx
            .ed
            .msg
            .set("Nothing cut: select a brick and press Ctrl+X first.".into());
    };
    let here = cx.level().path.clone();
    let into = match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::Brick(b) if !cx.m.g.bricks[b].ghost => Some(cx.m.g.bricks[b].name.clone()),
        _ => None,
    };
    let (path, place) = match into {
        Some(n) if here == c.level && n == c.name => {
            return cx.ed.msg.set(format!("Cannot paste {n} into itself."));
        }
        Some(n) => (brick_path(&here, &n), format!("into {n}")),
        None if here == c.level => {
            return cx.ed.msg.set(format!(
                "{} is already on this level: open another level, or select a brick to paste into.",
                c.name
            ));
        }
        None => (level_brick_path(&here), format!("to {here}")),
    };
    cx.ed.clip.set(None);
    move_brick(cx, &c.level, c.name, path, place);
}
