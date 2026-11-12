//! The ops log: every op of the change, newest first; agent ops are styled apart and the
//! proposed ones get Accept / Reject with a note.

use super::send_reload;
use crate::state::*;
use brickie_web_api::change::{By, OpRecord, State, Verdict};
use brickie_web_api::{ChangeReview, ChangeReviewRequest};
use sycamore::prelude::*;

fn state_name(s: State) -> &'static str {
    match s {
        State::Proposed => "proposed",
        State::Accepted => "accepted",
        State::Rejected => "rejected",
    }
}

fn review(ed: Edit, seq: u64) -> View {
    let note = create_signal(String::new());
    let go = move |verdict: Verdict| {
        move |_: Click| {
            let n = note.get_clone();
            let req = ChangeReviewRequest {
                seq,
                verdict,
                note: (!n.trim().is_empty()).then_some(n),
            };
            send_reload::<ChangeReview>(ed, req);
        }
    };
    view! {
        div(class="review") {
            input(r#type="text", placeholder="note (why)", bind:value=note)
            button(class="go", on:click=go(Verdict::Accept)) { "Accept" }
            button(class="danger", on:click=go(Verdict::Reject)) { "Reject" }
        }
    }
}

fn entry(ed: Edit, r: &OpRecord) -> View {
    let by = if r.by == By::Agent { "agent" } else { "human" };
    let st = state_name(r.state);
    let cls = format!(
        "oprec {by} {st}{}",
        if r.conflict.is_some() {
            " conflict"
        } else {
            ""
        }
    );
    let head = format!("#{} {} {}", r.seq, r.op.name(), r.key);
    let file = r.file.clone();
    let note = r.note.clone().map(|n| view! { div(class="muted") { (n) } });
    let reviewed = r.review.as_ref().map(|v| {
        let t = format!(
            "{st} by {:?}{}",
            v.by,
            v.note.as_ref().map_or(String::new(), |n| format!(": {n}"))
        );
        view! { div(class="muted") { (t) } }
    });
    let conflict = r
        .conflict
        .clone()
        .map(|c| view! { div(class="err-text") { "conflict: " (c) } });
    let act = (r.by == By::Agent && r.state == State::Proposed).then(|| review(ed, r.seq));
    view! {
        div(class=cls) {
            div(class="ophead") { span(class=format!("chip {by}")) { (by) } b { (head) } span(class=format!("chip {st}")) { (st) } }
            div(class="mono muted") { (file) }
            (note) (reviewed) (conflict) (act)
        }
    }
}

/// The ops log tab.
pub fn log(cx: &Cx) -> View {
    let Some(ch) = cx.m.change.as_ref() else {
        return View::default();
    };
    if ch.log.is_empty() {
        return view! { p(class="muted") { "No ops yet. Draw a brick from the palette." } };
    }
    let items: Vec<View> = ch.log.iter().rev().map(|r| entry(cx.ed, r)).collect();
    View::from(items)
}
