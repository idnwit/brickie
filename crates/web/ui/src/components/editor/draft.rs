//! The draft's part of the design bar: which version is published, whether a draft exists and
//! what it forked from, whether it is behind (rebase), the publish gate (blocking items, listed
//! in the chip's tip) and the Publish / Rebase / Discard actions. Each action is based on the
//! version on screen and never retried on a newer head.

use super::act;
use crate::state::*;
use brickie_web_api::{DesignDiscard, DesignPublish, DesignRebase, DesignState, DraftRequest};
use brickie_web_api::{PublishRequest, design::Gate};
use sycamore::prelude::*;

/// The gate chip: ready, or the number of blocking items with each one in the tip.
fn gate_chip(g: &Gate) -> View {
    if g.ok() {
        let tip = "The code matches the draft's structure: it can be published";
        return view! { span(class="chip ready", title=tip) { "ready to publish" } };
    }
    let mut tip = String::from("The code must match the draft before it is published:");
    for b in &g.blocking {
        tip.push_str("\n\u{2022} ");
        tip.push_str(&b.message);
    }
    let text = format!("{} blocking", g.blocking.len());
    view! { span(class="chip conflict", title=tip) { (text) } }
}

fn button(label: &'static str, tip: String, off: bool, run: impl Fn() + 'static) -> View {
    view! { button(title=tip, disabled=off, on:click=move |_: Click| run()) { (label) } }
}

/// The draft segment of the bar.
pub fn draft_bar(cx: &Cx, ch: &DesignState) -> View {
    let pubd = format!("published v{}", ch.published);
    let tip = "What `brickie check` and the view mode read";
    let published = view! { span(class="chip", title=tip) { (pubd) } };
    let Some(fork) = ch.forked_from else {
        let tip = "No draft: the next edit starts one; publish copies it into the real files";
        return view! { (published) span(class="muted", title=tip) { "no draft" } };
    };
    let base = ch.v;
    let draft = format!("draft v{base} \u{00B7} from v{fork}");
    let state = match (ch.behind(), &ch.gate) {
        (true, _) => {
            let tip = format!(
                "v{} was published since the draft forked: rebase",
                ch.published
            );
            view! { span(class="chip conflict", title=tip) { "behind: rebase" } }
        }
        (false, Some(g)) => gate_chip(g),
        (false, None) => View::default(),
    };
    let ready = !ch.behind() && ch.gate.as_ref().is_some_and(Gate::ok);
    let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
    let publish = button(
        "Publish",
        "Copy the draft into the brickie.toml files as one published version".into(),
        !ready,
        move || {
            let req = PublishRequest {
                base,
                ..Default::default()
            };
            act::<DesignPublish>(&c1, req, "published".into());
        },
    );
    let rebase = button(
        "Rebase",
        format!("Replay the draft's edits onto published v{}", ch.published),
        !ch.behind(),
        move || act::<DesignRebase>(&c2, DraftRequest { base }, "rebased".into()),
    );
    let discard = button(
        "Discard",
        "Drop the draft (the history keeps it)".into(),
        false,
        move || {
            if crate::platform::confirm("Discard the draft? Its edits are dropped.") {
                act::<DesignDiscard>(&c3, DraftRequest { base }, "draft discarded".into());
            }
        },
    );
    view! {
        (published)
        span(class="chip by-human") { (draft) }
        (state)
        span(class="seg") { (publish) (rebase) (discard) }
    }
}
