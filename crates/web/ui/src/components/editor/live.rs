//! Writes made elsewhere (a coding agent's `brickie change propose`, the CLI, another tab)
//! reach the page through the server's event stream (`/api/events`): a `change` event fetches
//! the change and the levels and patches them in place (toast "Agent proposed N ops"); `cache`
//! and `config` raise a "refresh" pill; `measure` drives the header's Measure button. Without
//! `EventSource` the page falls back to polling the change every 3 s.

use super::{keep, toast};
use crate::components::call::{call, get};
use crate::platform;
use crate::state::*;
use brickie_web_api::{
    ChangeLevels, ChangeLevelsRequest, ChangeState, ChangeView, EVENTS_PATH, ServerEvent,
};
use std::cell::RefCell;

const EVERY_MS: i32 = 3000;
/// Retry delay of a refresh that waits for the user to finish a gesture.
const BUSY_MS: i32 = 400;

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
        && ed.draft.with(Option::is_none)
        && ed.rename.with(Option::is_none)
        && ed.lane_ask.with(Option::is_none)
        && ed.carry.with(Option::is_none)
        && !platform::typing()
}

/// Fetch the change; when its head moved, fetch the levels and patch the page. `done(true)`
/// when the page was quiet enough to look.
fn refresh(done: impl FnOnce(bool) + 'static) {
    let Some(cx) = current_cx() else {
        return done(false);
    };
    let Some(before) = cx.m.change.clone().filter(|_| quiet(&cx)) else {
        return done(false);
    };
    get::<ChangeState>(move |r| {
        let Ok(Some(now)) = r else { return done(true) };
        if now.status.head == before.status.head {
            return done(true);
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
            done(true);
        });
    });
}

/// Refresh now, or again shortly while the user is in the middle of something.
fn refresh_soon() {
    refresh(|looked| {
        if !looked && current_cx().is_some_and(|c| c.editing()) {
            platform::after(BUSY_MS, refresh_soon);
        }
    });
}

fn poll() {
    refresh(|_| platform::after(EVERY_MS, poll));
}

thread_local! {
    static SEEN: RefCell<Seen> = RefCell::new(Seen::default());
}

fn handle(bus: Bus, data: String) {
    let Ok(ev) = serde_json::from_str::<ServerEvent>(&data) else {
        return;
    };
    let change = current_cx()
        .and_then(|c| c.m.change.as_ref().map(|ch| (true, ch.status.head)))
        .unwrap_or((false, 0));
    match SEEN.with(|s| on_event(&mut s.borrow_mut(), &ev, change)) {
        Action::RefreshChange => refresh_soon(),
        Action::Pill(text) => bus.pill.set(text.to_string()),
        Action::Job(j) => bus.job.set(j),
        Action::Nothing => {}
    }
}

/// Listen to the server's events (once per page, browser only); poll when that fails.
pub fn start(bus: Bus) {
    let kinds = ["change", "cache", "measure", "config"];
    if !platform::on_events(EVENTS_PATH, &kinds, move |d| handle(bus, d)) {
        platform::after(EVERY_MS, poll);
    }
}
