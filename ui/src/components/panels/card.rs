//! A diagnostic as a card (the same fields `brickie check` prints) with Accept / Unaccept.

use crate::model::*;
use crate::platform;
use crate::state::*;
use sycamore::prelude::*;

const MAX_LISTED: usize = 60;

fn span_link(modal: Signal<Option<String>>, s: &SpanView) -> View {
    src_link(modal, s.href(), s.loc(), "mono")
}

pub fn card_view(c: Card, modal: Signal<Option<String>>) -> View {
    let d = c.d.clone();
    let acc = c.accepted;
    let base = format!("diag {}", d.level);
    let cls = move || {
        if acc.get() {
            format!("{base} accepted")
        } else {
            base.clone()
        }
    };
    let id = format!("d-{}", d.id);
    let badge = format!(
        "{}[{}]",
        if d.level == "error" {
            "error"
        } else {
            "warning"
        },
        d.code
    );
    let message = d.message.clone();

    let primary = match &d.span {
        Some(s) => {
            let label = s.label.clone().unwrap_or_default();
            let link = span_link(modal, s);
            view! { div(class="kv") { b { "at" } (link) " " span(class="muted") { (label) } } }
        }
        None => View::default(),
    };
    let snippet = match &d.snippet {
        Some(s) => {
            let (n, before, hit, after) = (
                s.line.to_string(),
                s.before.clone(),
                s.hit.clone(),
                s.after.clone(),
            );
            view! { pre(class="snip") { span(class="muted") { (n) " | " } (before) mark { (hit) } (after) } }
        }
        None => View::default(),
    };
    let rule = match &d.rule {
        Some(r) => {
            let text = match &r.loc {
                Some(l) => format!("{} ({l})", r.name),
                None => r.name.clone(),
            };
            view! { div(class="kv") { b { "rule" } (text) } }
        }
        None => View::default(),
    };
    let help = match &d.help {
        Some(h) => {
            let h = h.clone();
            view! { div(class="kv") { b { "help" } (h) } }
        }
        None => View::default(),
    };
    let notes: Vec<View> = d
        .notes
        .iter()
        .map(|n| {
            let text = n.text.clone();
            let link = n
                .span
                .as_ref()
                .map(|s| span_link(modal, s))
                .unwrap_or_default();
            view! { div(class="kv") { b { "note" } (text) " " (link) } }
        })
        .collect();
    let notes = View::from(notes);
    let more = if d.spans.is_empty() {
        View::default()
    } else {
        let summary = format!("{} more location(s)", d.spans.len());
        let mut links: Vec<View> = d
            .spans
            .iter()
            .take(MAX_LISTED)
            .map(|s| {
                let link = span_link(modal, s);
                view! { div { (link) } }
            })
            .collect();
        if d.spans.len() > MAX_LISTED {
            let rest = format!("and {} more", d.spans.len() - MAX_LISTED);
            links.push(view! { div(class="muted") { (rest) } });
        }
        let links = View::from(links);
        view! { details(class="kv") { summary { (summary) } (links) } }
    };
    let review = review_view(c);
    view! {
        article(id=id, class=cls) {
            div(class="head") { span(class="badge") { (badge) } span { (message) } }
            (primary)
            (snippet)
            (rule)
            (help)
            (notes)
            (more)
            (review)
        }
    }
}

fn review_view(c: Card) -> View {
    let Card {
        d,
        accepted,
        reason,
        input,
        note,
        busy,
    } = c;
    let id_accept = d.id.clone();
    let id_un = d.id;
    let accept = move |_: Click| {
        let r = input.get_clone().trim().to_string();
        if r.is_empty() {
            note.set("A reason is required to accept.".into());
            return;
        }
        busy.set(true);
        platform::post_review(
            ReviewRequest {
                id: id_accept.clone(),
                reason: Some(r),
            },
            move |res| {
                busy.set(false);
                match res {
                    Ok(rep) => {
                        reason.set(rep.reason.unwrap_or_default());
                        accepted.set(rep.accepted);
                        note.set(String::new());
                    }
                    Err(e) => note.set(e),
                }
            },
        );
    };
    let unaccept = move |_: Click| {
        busy.set(true);
        platform::post_review(
            ReviewRequest {
                id: id_un.clone(),
                reason: None,
            },
            move |res| {
                busy.set(false);
                match res {
                    Ok(rep) => {
                        accepted.set(rep.accepted);
                        note.set(String::new());
                    }
                    Err(e) => note.set(e),
                }
            },
        );
    };
    let body = move || {
        if accepted.get() {
            let why = format!("Accepted: {}", reason.get_clone());
            view! {
                span(class="muted") { (why) }
                button(on:click=unaccept.clone()) { "Unaccept" }
            }
        } else {
            view! {
                input(r#type="text", placeholder="reason (required)", bind:value=input)
                button(class="go", on:click=accept.clone()) { "Accept" }
            }
        }
    };
    let err = move || note.get_clone();
    view! { div(class="review") { (body) span(class="err-text") { (err) } } }
}
