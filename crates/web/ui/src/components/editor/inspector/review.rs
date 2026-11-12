//! The review bar at the top of the inspector for an element an agent op touches: the op in
//! plain words with Accept / Reject (a note is required to reject), or, for a conflict, Reject
//! and Re-apply as mine.

use super::super::keys::review;
use super::super::write;
use crate::state::*;
use brickie_web_api::change::{OpRecord, Verdict};
use brickie_web_api::{ChangeReapply, ChangeReapplyRequest};
use sycamore::prelude::*;

/// Re-record conflicting op `seq` as a human op against the current design.
pub fn reapply(cx: &Cx, seq: u64) {
    let req = ChangeReapplyRequest { seq };
    write::<ChangeReapply, _>(cx, req, format!("re-applied #{seq}"), |_, _| {});
}

fn bar(cx: &Cx, r: &OpRecord) -> View {
    let seq = r.seq;
    let note = create_signal(String::new());
    let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
    let accept = move |_: Click| review(&c1, seq, Verdict::Accept, None);
    let reject = move |_: Click| {
        let n = note.get_clone();
        if n.trim().is_empty() {
            c2.ed.msg.set("Rejecting needs a note (why).".into());
            crate::platform::focus("review-note");
            return;
        }
        review(&c2, seq, Verdict::Reject, Some(n));
    };
    let head = format!("Agent proposed: {}", describe(&r.op));
    let why = r
        .note
        .clone()
        .map(|n| view! { div(class="muted") { "\u{201C}" (n) "\u{201D}" } });
    let conflict = r.conflict.clone().map(|c| {
        let re = move |_: Click| reapply(&c3, seq);
        view! { div(class="err-text") { "Conflict: " (c) } button(id="reapply", on:click=re) { "Re-apply as mine" } }
    });
    let accept_btn = r
        .conflict
        .is_none()
        .then(|| view! { button(id="accept", class="go", on:click=accept) { "Accept (A)" } });
    view! {
        div(class="revbar") {
            b { (head) }
            (why)
            (conflict)
            input(id="review-note", r#type="text", placeholder="note (required to reject)", bind:value=note)
            div(class="row") { (accept_btn) button(id="reject", class="danger", on:click=reject) { "Reject (R)" } }
        }
    }
}

/// Review bars of the pending agent ops `seqs`.
pub fn review_bar(cx: &Cx, seqs: &[u64]) -> View {
    let Some(ch) = cx.m.change.as_deref() else {
        return View::default();
    };
    let bars: Vec<View> = ch
        .log
        .iter()
        .filter(|r| seqs.contains(&r.seq))
        .map(|r| bar(cx, r))
        .collect();
    View::from(bars)
}
