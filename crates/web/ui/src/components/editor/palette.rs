//! The editor's right panel: the palette of brick kinds (pick one, then click or drop on the
//! stage), and the Inspector / Ops log tabs.

use super::inspector::inspector;
use super::log::log;
use crate::state::*;
use brickie_web_api::change::{BrickKind, By, State};
use sycamore::prelude::*;

pub const KINDS: [(BrickKind, &str, &str); 3] = [
    (
        BrickKind::Crate,
        "Crate",
        "A crate: members default to path:crates/<name>/",
    ),
    (
        BrickKind::Module,
        "Module",
        "A module of a crate: members name it, crate::module",
    ),
    (
        BrickKind::Group,
        "Group",
        "A composite: gets its own brickie.toml; zoom in to add children",
    ),
];

pub fn kind_name(k: BrickKind) -> &'static str {
    KINDS
        .iter()
        .find(|(x, _, _)| *x == k)
        .map_or("brick", |(_, n, _)| n)
}

fn palette(ed: Edit) -> View {
    let items: Vec<View> = KINDS
        .iter()
        .map(|&(kind, name, tip)| {
            let cls = move || {
                let on = ed.tool.get() == Some(kind);
                format!("pal{}", if on { " on" } else { "" })
            };
            let down = move |e: Click| {
                e.prevent_default();
                let again = ed.tool.get() == Some(kind);
                ed.tool.set((!again).then_some(kind));
                ed.carry.set(!again);
                ed.link.set(None);
            };
            view! { button(class=cls, title=tip, "data-kind"=name, on:mousedown=down) { (name) } }
        })
        .collect();
    let items = View::from(items);
    view! {
        div(id="palette") {
            h4 { "Palette" }
            div(class="pals") { (items) }
            p(class="muted") { "Pick a kind, then click (or drop) on the canvas where it goes. Drag a brick onto another to move it inside." }
        }
    }
}

fn tabs(cx: &Cx) -> View {
    let ed = cx.ed;
    let proposed = cx.m.change.as_ref().map_or(0, |c| {
        c.log
            .iter()
            .filter(|r| r.by == By::Agent && r.state == State::Proposed)
            .count()
    });
    let tab = move |id: &'static str, label: String| {
        let cls = move || format!("tab{}", if ed.tab.get() == id { " on" } else { "" });
        view! { button(class=cls, on:click=move |_: Click| ed.tab.set(id)) { (label) } }
    };
    let log_label = if proposed > 0 {
        format!("Ops log ({proposed} to review)")
    } else {
        "Ops log".into()
    };
    let (a, b) = (tab("inspect", "Inspector".into()), tab("log", log_label));
    view! { div(class="tabs") { (a) (b) } }
}

/// The right panel in edit mode.
pub fn panel(cx: &Cx) -> View {
    let pal = palette(cx.ed);
    let tabs = tabs(cx);
    let c = cx.clone();
    let body = move || match c.ed.tab.get() {
        "log" => log(&c),
        _ => inspector(&c),
    };
    view! {
        aside(id="detail", class="editor") {
            (pal)
            (tabs)
            div(id="dcont") { (body) }
        }
    }
}
