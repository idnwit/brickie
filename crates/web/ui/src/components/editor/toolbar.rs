//! The floating toolbar at the top of the canvas (centred, collapsible, fixed while panning):
//! Select / Hand, the brick tiles (drag one onto the canvas, or click it then the canvas), the
//! link types (arm one: ports show on every brick), the scope of new rules and delete.

use crate::state::*;
use brickie_web_api::change::BrickKind;
use sycamore::prelude::*;

pub const KINDS: [(BrickKind, &str, &str, &str); 3] = [
    (
        BrickKind::Crate,
        "Crate",
        "\u{25A3}",
        "A crate: members default to path:crates/<name>/",
    ),
    (
        BrickKind::Module,
        "Module",
        "\u{25A4}",
        "A module of a crate: members name it, crate::module",
    ),
    (
        BrickKind::Group,
        "Group",
        "\u{25A6}",
        "A composite with its own brickie.toml; zoom in to add children",
    ),
];

pub fn kind_name(k: BrickKind) -> &'static str {
    KINDS.iter().find(|x| x.0 == k).map_or("brick", |x| x.1)
}

fn on(b: bool, base: &'static str) -> String {
    if b {
        format!("{base} on")
    } else {
        base.to_string()
    }
}

/// Stop the stage from panning or clearing the selection under the toolbar.
fn stop(e: Click) {
    e.stop_propagation();
}

fn tools(ed: Edit) -> View {
    let t = move |tool: Tool, label: &'static str, tip: &'static str| {
        let cls = move || on(ed.tool.get() == tool, "tb");
        view! { button(class=cls, title=tip, on:click=move |_: Click| ed.tool.set(tool)) { (label) } }
    };
    view! {
        (t(Tool::Select, "\u{2196}", "Select (V): click to select, drag bricks to move them"))
        (t(Tool::Hand, "\u{270B}", "Hand (H): drag anywhere to pan"))
    }
}

fn tiles(ed: Edit) -> View {
    let items: Vec<View> = KINDS
        .iter()
        .map(|&(kind, name, glyph, tip)| {
            let cls = move || on(ed.brick_tool.get() == Some(kind), "tb tile");
            let down = move |e: Click| {
                e.prevent_default();
                e.stop_propagation();
                ed.link_type.set(None);
                ed.carry
                    .set(Some((kind, e.client_x() as f64, e.client_y() as f64)));
            };
            let label = format!("{glyph} {name}");
            view! { button(class=cls, title=tip, "data-kind"=name, on:mousedown=down) { (label) } }
        })
        .collect();
    View::from(items)
}

fn links(ed: Edit) -> View {
    let items: Vec<View> = LINK_TYPES
        .iter()
        .map(|&ty| {
            let cls = move || on(ed.link_type.get() == Some(ty), "tb lt");
            let click = move |_: Click| {
                let again = ed.link_type.get() == Some(ty);
                ed.link_type.set((!again).then_some(ty));
                ed.brick_tool.set(None);
            };
            let label = format!("{} {}", ty.glyph(), ty.name());
            let tip = format!("{} link (L): show ports on every brick; drag a port onto a sibling or a tag lane", ty.name());
            view! { button(class=cls, "data-link"=ty.name(), style=format!("--lc:var(--l-{})", ty.name()), title=tip, on:click=click) { (label) } }
        })
        .collect();
    View::from(items)
}

fn scopes(ed: Edit) -> View {
    let items: Vec<View> = SCOPES
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let cls = move || on(ed.scope.get() == s, "tb sc");
            let tip = format!("Scope of new rules: {s} ({})", i + 1);
            view! { button(class=cls, title=tip, on:click=move |_: Click| ed.scope.set(s)) { (s) } }
        })
        .collect();
    View::from(items)
}

/// The toolbar (empty outside edit mode).
pub fn toolbar(cx: &Cx) -> View {
    let ed = cx.ed;
    let c = cx.clone();
    let del = move |_: Click| super::keys::delete_selection(&c);
    let body = move || {
        if !ed.toolbar_open.get() {
            return View::default();
        }
        let del = del.clone();
        view! {
            span(class="tgrp") { (tools(ed)) }
            span(class="tgrp") { (tiles(ed)) }
            span(class="tgrp") { (links(ed)) }
            span(class="tgrp seg") { (scopes(ed)) }
            button(class="tb", id="tb-del", title="Delete the selection (Del)", on:click=del) { "\u{232B}" }
        }
    };
    let fold = move |_: Click| ed.toolbar_open.update(|o| *o = !*o);
    let fold_label = move || {
        if ed.toolbar_open.get() {
            "\u{25B4}"
        } else {
            "\u{25BE} tools"
        }
    };
    view! {
        div(id="toolbar", on:mousedown=stop, on:click=stop, on:dblclick=stop) {
            (body)
            button(class="tb fold", title="Collapse / expand the toolbar", on:click=fold) { (fold_label) }
        }
    }
}
