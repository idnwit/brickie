//! The Ops tab: the change log as one timeline in seq order, in plain words, with author and state chips
//! and the file; filters (agent / human / proposed / conflict); a row selects its element on
//! the canvas; "Accept all from agent" (confirmed); conflicts offer Re-apply as mine.

use super::inspector::review_bar;
use super::widgets::seg;
use super::write;
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

/// Select the element of `r` on the canvas (its level, its brick).
fn show(cx: &Cx, r: &OpRecord) {
    let Some(ch) = cx.m.change.as_deref() else {
        return;
    };
    let level = op_level(ch, r).unwrap_or_else(|| "root".into());
    let brick = op_brick(&r.op).unwrap_or_default().to_string();
    cx.ed.rule.set(
        ch.planned_edges
            .iter()
            .find(|e| e.seq == Some(r.seq))
            .map(|e| RuleRef {
                file: e.file.clone(),
                index: e.rule,
            }),
    );
    cx.go(cx
        .params
        .with(|p| p.at_level(&level))
        .with(|p| p.brick = brick));
}

fn entry(cx: &Cx, r: &OpRecord) -> View {
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
    let head = format!("#{} {}", r.seq, describe(&r.op));
    let place = format!("{} \u{00B7} {}", r.key, r.file);
    let note = r.note.clone().map(|n| view! { div(class="muted") { (n) } });
    let (c, rec) = (cx.clone(), r.clone());
    let act = (r.by == By::Agent && r.state == State::Proposed).then(|| review_bar(cx, &[r.seq]));
    view! {
        div(class=cls) {
            div(class="ophead", on:click=move |_: Click| show(&c, &rec)) {
                span(class=format!("chip {by}")) { (by) } b { (head) } span(class=format!("chip {st}")) { (st) }
            }
            div(class="muted mono") { (place) } (note) (act)
        }
    }
}

/// Accept `seqs` one after the other (each reply patches the page before the next).
fn accept_chain(cx: &Cx, mut seqs: Vec<u64>) {
    let Some(seq) = seqs.pop() else { return };
    let req = ChangeReviewRequest {
        seq,
        verdict: Verdict::Accept,
        note: Some("accepted with all".into()),
    };
    write::<ChangeReview, _>(cx, req, format!("accepted #{seq}"), move |now, _| {
        accept_chain(now, seqs)
    });
}

fn accept_all(cx: &Cx) -> View {
    let ask = create_signal(false);
    let c = cx.clone();
    let go = move |_: Click| {
        ask.set(false);
        let mut seqs: Vec<u64> =
            c.m.change
                .as_deref()
                .map(|ch| {
                    ch.log
                        .iter()
                        .filter(|r| pending(r) && r.conflict.is_none())
                        .map(|r| r.seq)
                        .collect()
                })
                .unwrap_or_default();
        seqs.reverse();
        accept_chain(&c, seqs);
    };
    view! {
        (move || if ask.get() {
            let go = go.clone();
            view! { span { "Accept every agent proposal? " button(class="go", on:click=go) { "Yes" } button(on:click=move |_: Click| ask.set(false)) { "No" } } }
        } else {
            view! { button(on:click=move |_: Click| ask.set(true)) { "Accept all from agent" } }
        })
    }
}

pub fn log(cx: &Cx) -> View {
    let Some(ch) = cx.m.change.clone() else {
        return View::default();
    };
    let ed = cx.ed;
    let filter = ed.ops_filter.get();
    let filters = seg(
        &["all", "agent", "human", "proposed", "conflict"],
        filter,
        move |f| ed.ops_filter.set(f),
    );
    // One timeline in global seq order (oldest first), so the story reads top to bottom.
    let items: Vec<View> = ch
        .log
        .iter()
        .filter(|r| keep_op(filter, r))
        .map(|r| entry(cx, r))
        .collect();
    let empty = items
        .is_empty()
        .then(|| view! { p(class="muted") { "No ops here. Drag a brick from the toolbar." } });
    let all = ch.log.iter().any(pending).then(|| accept_all(cx));
    let items = View::from(items);
    view! { div(class="row") { (filters) } (all) (items) (empty) }
}
