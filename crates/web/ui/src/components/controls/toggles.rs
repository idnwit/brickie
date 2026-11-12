//! The data-layers dropdown (`Data layers`): edge, kind and finding-group toggles with their
//! sub-layer menus (`SUBS`). Any toggle switches the preset to Custom.

use crate::state::*;
use sycamore::prelude::*;

/// One checkbox of the data-layers menu.
pub fn layer_box(
    cx: &Cx,
    key: &'static str,
    label: &'static str,
    parent: Option<&'static str>,
) -> View {
    let (c1, c2) = (cx.clone(), cx.clone());
    let checked = move || Lay::from_params(&c1.params.get_clone()).on(key);
    let disabled = move || parent.is_some_and(|g| !Lay::from_params(&c2.params.get_clone()).on(g));
    let c = cx.clone();
    view! {
        label {
            input(r#type="checkbox", "data-l"=key, prop:checked=checked, disabled=disabled, on:change=move |_: web_sys::Event| {
                let p = with_toggle(&c.params.get_clone(), key);
                c.go(p);
            })
            (label)
        }
    }
}

/// A group checkbox with its own dropdown of sub-layers (`SUBS`).
fn group_menu(cx: &Cx, key: &'static str, label: &'static str) -> View {
    let subs = SUBS
        .iter()
        .find(|(g, _)| *g == key)
        .map_or(&[][..], |(_, s)| *s);
    let items: Vec<View> = subs
        .iter()
        .map(|(k, t)| layer_box(cx, k, t, Some(key)))
        .collect();
    let items = View::from(items);
    let (c1, c2) = (cx.clone(), cx.clone());
    let id = format!("dd-{key}");
    let id2 = id.clone();
    let hidden = move || {
        if c1.menu.get_clone() == id {
            "ddm"
        } else {
            "ddm off"
        }
    };
    let toggle = layer_box(cx, key, label, None);
    view! {
        span(class="dd") {
            (toggle)
            button(class="ddb", title="sub-layers", on:click=move |e: Click| {
                e.stop_propagation();
                let open = c2.menu.get_clone() == id2;
                c2.menu.set(if open { String::new() } else { id2.clone() });
            }) { "\u{25BE}" }
            div(class=hidden) { (items) }
        }
    }
}

/// The dropdown button and its menu.
pub fn layers_menu(cx: &Cx) -> View {
    let menu_open = {
        let c = cx.clone();
        move || {
            if c.menu.get_clone() == "layers" {
                "ddm lay-pop"
            } else {
                "ddm lay-pop off"
            }
        }
    };
    let c = cx.clone();
    let toggle_layers = move |e: Click| {
        e.stop_propagation();
        let open = c.menu.get_clone() == "layers";
        c.menu
            .set(if open { String::new() } else { "layers".into() });
    };
    let edges = View::from(vec![
        layer_box(cx, "edges", "edges", None),
        layer_box(cx, "tests", "tests", None),
    ]);
    let kinds = View::from(vec![
        layer_box(cx, "k-uses", "uses", None),
        layer_box(cx, "k-implements", "implements", None),
        layer_box(cx, "k-reexports", "reexports", None),
    ]);
    let viol = View::from(vec![
        group_menu(cx, "deps", "deps"),
        group_menu(cx, "cx", "complexity"),
        group_menu(cx, "stale", "stale"),
        group_menu(cx, "iface", "interface"),
    ]);
    view! {
        span(class="dd") {
            button(class="ddb", on:click=toggle_layers) { "Data layers \u{25BE}" }
            div(class=menu_open, on:click=move |e: Click| e.stop_propagation()) {
                span(class="tg") { span(class="tgl") { "Edges" } (edges) }
                span(class="tg") { span(class="tgl") { "Kinds" } (kinds) }
                span(class="tg") { span(class="tgl") { "Findings" } (viol) }
            }
        }
    }
}
