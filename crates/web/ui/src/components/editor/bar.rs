//! The change bar (in place of the banner while a change is open): intent, status chips,
//! Preview/Current, Publish and Discard. Without a change, the header's "Design" button opens
//! one.

use super::dialogs::dialog;
use crate::state::*;
use brickie_web_api::{ChangeView, Params};
use sycamore::prelude::*;

/// The header button that opens a new change (only when none is open).
pub fn design_button(cx: &Cx) -> View {
    if cx.m.change.is_some() {
        return View::default();
    }
    let ed = cx.ed;
    let d = dialog(ed);
    view! {
        button(id="design", title="Open a design change: draw bricks and rules, publish them to brickie.toml",
            on:click=move |_: Click| ed.dialog.set(Dialog::Open)) { "Design" }
        (d)
    }
}

fn chip(text: String, cls: &'static str) -> View {
    view! { span(class=format!("chip {cls}")) { (text) } }
}

fn chips(ch: &ChangeView) -> View {
    let s = &ch.status;
    let mut out = vec![chip(
        format!("{} ops", s.accepted + s.proposed + s.rejected),
        "",
    )];
    if s.proposed > 0 {
        out.push(chip(format!("{} proposed", s.proposed), "agent"));
    }
    if s.conflicts > 0 {
        out.push(chip(format!("{} conflicts", s.conflicts), "conflict"));
    }
    let planned = ch
        .todo
        .iter()
        .filter(|t| t.kind == brickie_web_api::change::TaskKind::Planned)
        .count();
    if planned > 0 {
        out.push(chip(format!("{planned} planned"), "planned"));
    }
    if let Some(e) = ch.error.clone() {
        out.push(view! { span(class="chip conflict", title=e) { "preview does not load" } });
    }
    View::from(out)
}

/// Preview (`on`) or Current: a full page load of the other design.
fn seg(label: &'static str, on: bool, cx: &Cx) -> View {
    let c = cx.clone();
    let cls = move || {
        let now = c.params.with(|p| p.editing());
        format!("segb{}", if now == on { " on" } else { "" })
    };
    let target =
        move |p: &Params| p.with(|p| p.change = if on { "1".into() } else { String::new() });
    let href = cx.params.with(target).url();
    let c = cx.clone();
    let go = move |e: Click| {
        e.prevent_default();
        crate::platform::goto(&c.params.with(target).url());
    };
    view! { a(class=cls, href=href, on:click=go) { (label) } }
}

/// The bar of the open change.
pub fn change_bar(cx: &Cx) -> View {
    let Some(ch) = cx.m.change.clone() else {
        return View::default();
    };
    let ed = cx.ed;
    let (prev, cur) = (seg("Preview", true, cx), seg("Current", false, cx));
    let intent = ch.status.intent.clone();
    let tip = intent.clone();
    let chips = chips(&ch);
    let msg = move || ed.msg.get_clone();
    let d = dialog(ed);
    view! {
        div(id="cbar") {
            span(class="muted") { "change" }
            b(class="intent", title=tip) { (intent) }
            (chips)
            span(class="seg") { (prev) (cur) }
            span(class="err-text") { (msg) }
            button(class="go", on:click=move |_: Click| {
                ed.dialog.set(Dialog::Publish(None));
                super::dialogs::load_plan(ed);
            }) { "Publish" }
            button(class="danger", on:click=move |_: Click| ed.dialog.set(Dialog::Discard)) { "Discard" }
            (d)
        }
    }
}
