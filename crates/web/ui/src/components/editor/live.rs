//! Writes made elsewhere (a coding agent's `brickie design ...`, a hand edit of the files,
//! another tab) reach the page through the server's event stream (`/api/events`): a `design`
//! event whose version differs from the one on screen fetches the design and the levels and
//! patches them in place, with a toast naming what changed and an Undo for another writer's
//! version. `cache` and `config` raise a "refresh" pill; `measure` drives the header's Measure
//! button. Without `EventSource` the page polls the design every 3 s.

use super::{keep, toast_with};
use crate::components::call::{call, get};
use crate::platform;
use crate::state::*;
use brickie_web_api::design::By;
use brickie_web_api::{
    DesignGet, DesignLevels, DesignState, EVENTS_PATH, LevelsRequest, ServerEvent,
};
use std::cell::RefCell;

const EVERY_MS: i32 = 3000;
/// Retry delay of a refresh that waits for the user to finish a gesture.
const BUSY_MS: i32 = 400;

/// The toast for the versions `now` has after `before`: what the latest one by another writer
/// (an agent or a hand edit) did, and that version to undo. `None` when only the human wrote.
pub fn changed_by_other(before: u64, now: &DesignState) -> Option<(String, u64)> {
    let v = (now.history.iter().rev())
        .take_while(|v| v.v > before)
        .find(|v| v.by != By::Human)?;
    let what = v.diff.first().cloned().unwrap_or_else(|| version_words(v));
    Some((format!("{} changed: {what}", by_word(v.by)), v.v))
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

/// Fetch the design; when its version moved, fetch the levels and patch the page. `done(true)`
/// when the page was quiet enough to look.
fn refresh(done: impl FnOnce(bool) + 'static) {
    let Some(cx) = current_cx() else {
        return done(false);
    };
    let Some(before) = cx.m.design.as_ref().map(|d| d.v).filter(|_| quiet(&cx)) else {
        return done(false);
    };
    get::<DesignGet>(move |r| {
        let Ok(now) = r else { return done(true) };
        if now.v == before {
            return done(true);
        }
        call::<DesignLevels>(&LevelsRequest::default(), move |r| {
            if let (Ok(levels), Some(cx)) = (r, current_cx()) {
                if let Some((text, v)) = changed_by_other(before, &now) {
                    toast_with(cx.ed, text, Some(v));
                }
                super::forget_indices(cx.ed);
                cx.live.apply(now, levels, Some(keep(&cx)));
            }
            done(true);
        });
    });
}

/// Refresh now, or again shortly while the user is in the middle of something.
pub(super) fn refresh_soon() {
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
    let on_screen = current_cx().and_then(|c| c.m.design.as_ref().map(|d| d.v));
    match SEEN.with(|s| on_event(&mut s.borrow_mut(), &ev, on_screen)) {
        Action::RefreshDesign => refresh_soon(),
        Action::Pill(text) => bus.pill.set(text.to_string()),
        Action::Job(j) => bus.job.set(j),
        Action::Nothing => {}
    }
}

/// Listen to the server's events (once per page, browser only); poll when that fails.
pub fn start(bus: Bus) {
    let kinds = ["design", "cache", "measure", "config"];
    if !platform::on_events(EVENTS_PATH, &kinds, move |d| handle(bus, d)) {
        platform::after(EVERY_MS, poll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_web_api::design::Version;

    fn ver(v: u64, by: By) -> Version {
        Version {
            v,
            hash: String::new(),
            prev: None,
            by,
            at: 0,
            note: None,
            ops: vec![],
            diff: vec![format!("+ brick b{v}")],
            files: vec![],
            removed: vec![],
            full: false,
        }
    }

    #[test]
    fn another_writers_version_is_named_with_undo() {
        let mut d = crate::state::tests_design();
        d.history = vec![ver(4, By::Human), ver(5, By::Agent), ver(6, By::Human)];
        assert_eq!(
            changed_by_other(4, &d),
            Some(("agent changed: + brick b5".into(), 5))
        );
        assert_eq!(changed_by_other(5, &d), None);
    }
}
