//! The header's "Design" toggle (`E`) and the change bar (in place of the banner while a change
//! is open): intent, status chips that act (proposed / conflicts -> Ops tab, planned -> select
//! the planned bricks, no reason -> Ops tab), Preview | Current | Diff, Publish and Discard.

use super::dialogs::dialog;
use super::keys::toggle_design;
use crate::state::*;
use brickie_web_api::ChangeView;
use brickie_web_api::change::TaskKind;
use sycamore::prelude::*;

/// The header button: opens a change (none open) or toggles design mode.
pub fn design_button(cx: &Cx) -> View {
    let on = cx.editing();
    let c = cx.clone();
    let cls = if on { "on" } else { "" };
    let tip = "Design (E): draw bricks and rules on the canvas; publish writes brickie.toml";
    let d = if cx.m.change.is_none() {
        dialog(cx.ed)
    } else {
        View::default()
    };
    view! {
        button(id="design", class=cls, title=tip, on:click=move |_: Click| toggle_design(&c)) { "Design" }
        (d)
    }
}

fn count(ch: &ChangeView, k: TaskKind) -> usize {
    ch.todo.iter().filter(|t| t.kind == k).count()
}

/// A status chip that acts when clicked.
fn chip(text: String, cls: &'static str, act: impl Fn() + 'static) -> View {
    view! { button(class=format!("chip {cls}"), on:click=move |_: Click| act()) { (text) } }
}

fn to_ops(ed: Edit, filter: &'static str) -> impl Fn() {
    move || {
        ed.tab.set("ops");
        ed.ops_filter.set(filter);
    }
}

fn chips(cx: &Cx, ch: &ChangeView) -> View {
    let (s, ed) = (&ch.status, cx.ed);
    let mut out = vec![chip(
        format!("{} ops", s.accepted + s.proposed + s.rejected),
        "",
        to_ops(ed, "all"),
    )];
    if s.proposed > 0 {
        out.push(chip(
            format!("{} proposed", s.proposed),
            "agent",
            to_ops(ed, "proposed"),
        ));
    }
    if s.conflicts > 0 {
        out.push(chip(
            format!("{} conflicts", s.conflicts),
            "conflict",
            to_ops(ed, "conflict"),
        ));
    }
    let planned = count(ch, TaskKind::Planned);
    if planned > 0 {
        let c = cx.clone();
        out.push(chip(format!("{planned} planned"), "planned", move || {
            let bricks: Vec<usize> = (0..c.m.g.bricks.len())
                .filter(|&i| c.m.g.bricks[i].planned)
                .collect();
            c.select(Sel::Group {
                tags: vec![],
                bricks,
            });
        }));
    }
    let reasons = count(ch, TaskKind::Reason);
    if reasons > 0 {
        let c = cx.clone();
        out.push(chip(
            format!("{reasons} without reason"),
            "planned",
            move || c.ed.tab.set("rules"),
        ));
    }
    if let Some(e) = ch.error.clone() {
        out.push(view! { span(class="chip conflict", title=e) { "preview does not load" } });
    }
    View::from(out)
}

/// Preview / Current / Diff.
fn segs(cx: &Cx) -> View {
    let ed = cx.ed;
    let item = move |label: &'static str, key: &'static str| {
        let c = cx.clone();
        let cls = move || {
            let on = match key {
                "p" => c.editing() && !ed.diff.get(),
                "c" => !c.editing(),
                _ => c.editing() && ed.diff.get(),
            };
            format!("segb{}", if on { " on" } else { "" })
        };
        let c = cx.clone();
        let tip = format!("{label} ({})", key.to_uppercase());
        view! { button(class=cls, title=tip, on:click=move |_: Click| { super::key(&c, key); }) { (label) } }
    };
    let (p, c, d) = (
        item("Preview", "p"),
        item("Current", "c"),
        item("Diff", "d"),
    );
    view! { span(class="seg") { (p) (c) (d) } }
}

/// The bar of the open change.
pub fn change_bar(cx: &Cx) -> View {
    let Some(ch) = cx.m.change.clone() else {
        return View::default();
    };
    let ed = cx.ed;
    let intent = ch.status.intent.clone();
    let tip = intent.clone();
    let (chips, segs) = (chips(cx, &ch), segs(cx));
    let age = ch.status.cache_age_secs.map_or(String::new(), |s| {
        format!("code measured {} min ago", s / 60)
    });
    let d = dialog(ed);
    let undo = undo_redo(cx, &ch);
    view! {
        div(id="cbar") {
            span(class="muted") { "change" }
            b(class="intent", title=tip) { (intent) }
            (chips)
            (segs)
            span(class="muted") { (age) }
            (undo)
            span(class="err-text") { (move || ed.msg.get_clone()) }
            button(class="go", on:click=move |_: Click| {
                ed.dialog.set(Dialog::Publish(None));
                super::dialogs::load_plan(ed);
            }) { "Publish\u{2026}" }
            button(class="danger", on:click=move |_: Click| ed.dialog.set(Dialog::Discard)) { "Discard" }
            (d)
        }
    }
}

/// Undo / Redo of the human's own ops, each naming what it would do.
fn undo_redo(cx: &Cx, ch: &ChangeView) -> View {
    let groups = cx.ed.groups.get_clone();
    let (u, r) = (undo_target(ch), redo_target(ch));
    let (ut, rt) = (undo_tip("Undo", u, &groups), undo_tip("Redo", r, &groups));
    let (uoff, roff) = (u.is_none(), r.is_none());
    let (c1, c2) = (cx.clone(), cx.clone());
    view! {
        span(class="seg") {
            button(id="undo", title=format!("{ut} (Ctrl+Z)"), disabled=uoff, on:click=move |_: Click| super::keys::retract(&c1)) { "\u{21B6} Undo" }
            button(id="redo", title=format!("{rt} (Ctrl+Shift+Z)"), disabled=roff, on:click=move |_: Click| super::keys::redo(&c2)) { "Redo \u{21B7}" }
        }
    }
}
