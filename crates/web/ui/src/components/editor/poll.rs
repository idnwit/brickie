//! Ops written elsewhere (the coding agent's `brickie change propose`, the CLI) reach the open
//! editor by polling: when the change's head moves, the levels are fetched and patched in place
//! with a toast "Agent proposed N ops".

use super::{keep, toast};
use crate::components::call::{call, get};
use crate::platform;
use crate::state::*;
use brickie_web_api::{ChangeLevels, ChangeLevelsRequest, ChangeState, ChangeView};

const EVERY_MS: i32 = 3000;

/// New pending agent ops in `now` that `before` did not have.
pub fn new_proposals(before: &ChangeView, now: &ChangeView) -> usize {
    now.log
        .iter()
        .filter(|r| pending(r) && r.seq > before.status.head)
        .count()
}

/// Nothing in progress that a re-render would drop (a drag, a name being typed, a request).
fn quiet(cx: &Cx) -> bool {
    let ed = cx.ed;
    cx.editing()
        && !ed.busy.get()
        && cx.op.get().is_none()
        && ed.connect.with(Option::is_none)
        && ed.draft.get().is_none()
        && ed.rename.with(Option::is_none)
        && !platform::typing()
}

fn tick() {
    let Some(cx) = current_cx() else {
        return again();
    };
    let Some(before) = cx.m.change.clone().filter(|_| quiet(&cx)) else {
        return again();
    };
    get::<ChangeState>(move |r| {
        let Ok(Some(now)) = r else { return again() };
        if now.status.head == before.status.head {
            return again();
        }
        let n = new_proposals(&before, &now);
        call::<ChangeLevels>(&ChangeLevelsRequest::default(), move |r| {
            if let (Ok(levels), Some(cx)) = (r, current_cx()) {
                if n > 0 {
                    toast(
                        cx.ed,
                        format!("Agent proposed {n} op(s): N jumps to the next"),
                    );
                }
                super::forget_indices(cx.ed);
                cx.live.apply(now, levels, Some(keep(&cx)));
            }
            again();
        });
    });
}

fn again() {
    platform::after(EVERY_MS, tick);
}

/// Start polling (once per page, browser only).
pub fn start() {
    again();
}
