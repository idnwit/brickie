//! A diagnostic as a card (the same fields `brickie check` prints) with Accept / Unaccept.

use crate::components::call;
use crate::state::*;
use brickie_web_api::*;
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
    let badge = badge_text(&d.level, &d.code);
    let message = d.message.clone();
    let primary = primary_view(&d, modal);
    let snippet = snippet_view(&d);
    let rule = rule_view(&d);
    let help = help_view(&d);
    let notes = notes_view(&d, modal);
    let more = more_view(&d, modal);
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

/// The badge text: `error[code]`, every other level is a `warning[code]`.
fn badge_text(level: &str, code: &str) -> String {
    format!(
        "{}[{code}]",
        if level == "error" { "error" } else { "warning" }
    )
}

/// The `at` line: the primary span and its label.
fn primary_view(d: &DiagView, modal: Signal<Option<String>>) -> View {
    match &d.span {
        Some(s) => {
            let label = s.label.clone().unwrap_or_default();
            let link = span_link(modal, s);
            view! { div(class="kv") { b { "at" } (link) " " span(class="muted") { (label) } } }
        }
        None => View::default(),
    }
}

/// The source snippet with the hit marked.
fn snippet_view(d: &DiagView) -> View {
    match &d.snippet {
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
    }
}

/// The violated rule and where it is declared.
fn rule_view(d: &DiagView) -> View {
    match &d.rule {
        Some(r) => {
            let text = match &r.loc {
                Some(l) => format!("{} ({l})", r.name),
                None => r.name.clone(),
            };
            view! { div(class="kv") { b { "rule" } (text) } }
        }
        None => View::default(),
    }
}

/// The help line.
fn help_view(d: &DiagView) -> View {
    match &d.help {
        Some(h) => {
            let h = h.clone();
            view! { div(class="kv") { b { "help" } (h) } }
        }
        None => View::default(),
    }
}

/// One line per note, with its span when it has one.
fn notes_view(d: &DiagView, modal: Signal<Option<String>>) -> View {
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
    View::from(notes)
}

/// The collapsed list of further locations, at most `MAX_LISTED`.
fn more_view(d: &DiagView, modal: Signal<Option<String>>) -> View {
    if d.spans.is_empty() {
        return View::default();
    }
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
        call::<Review>(
            &ReviewRequest {
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
        call::<Review>(
            &ReviewRequest {
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

#[cfg(test)]
mod tests {
    use super::badge_text;

    #[test]
    fn badge_names_errors_and_calls_the_rest_warnings() {
        assert_eq!(badge_text("error", "deps/deny"), "error[deps/deny]");
        assert_eq!(
            badge_text("warn", "complexity/lines"),
            "warning[complexity/lines]"
        );
    }
}
