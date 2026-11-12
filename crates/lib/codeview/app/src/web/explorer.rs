//! The explorer: a view switcher over the host's trees, the filter chip, and the tree with
//! badges (errors and warnings).

use super::{St, actions};
use crate::logic;
use codeview_port::{Badge, Node, NodeKind};
use sycamore::prelude::*;
use web_sys::MouseEvent;

fn badge(b: &Option<Badge>) -> View {
    let Some(b) = b else {
        return View::default();
    };
    let mut parts: Vec<View> = Vec::new();
    if b.errors > 0 {
        let t = format!("{}E", b.errors);
        parts.push(view! { span(class="bd err") { (t) } });
    }
    if b.warns > 0 {
        let t = format!("{}W", b.warns);
        parts.push(view! { span(class="bd warn") { (t) } });
    }
    View::from(parts)
}

/// The marker glyph of a node kind; folders and files need none.
fn glyph(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Composite => "\u{25c8}",
        NodeKind::Brick => "\u{25c6}",
        NodeKind::Crate => "\u{25a3}",
        _ => "",
    }
}

fn kind_class(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Group => "k-group",
        NodeKind::Brick => "k-brick",
        NodeKind::Composite => "k-composite",
        NodeKind::Crate => "k-crate",
        NodeKind::Folder => "k-folder",
        NodeKind::File => "k-file",
    }
}

/// The tag chips of a node, each coloured by a hue derived from its text.
fn chips(tags: &[String]) -> View {
    View::from(
        tags.iter()
            .map(|t| {
                let style = format!("--h:{}", logic::tag_hue(t));
                let t = t.clone();
                view! { span(class="tg", style=style) { (t) } }
            })
            .collect::<Vec<_>>(),
    )
}

fn node(st: St, n: Node, depth: usize) -> View {
    let pad = format!("padding-left:{}px", 6 + depth * 12);
    let label = n.label.clone();
    let tip = n.path.clone().unwrap_or_else(|| n.id.clone());
    let b = badge(&n.badge);
    let (kc, mark, tg) = (kind_class(n.kind), glyph(n.kind), chips(&n.tags));
    if let Some(path) = n.path {
        let p = path.clone();
        let cls = move || {
            if st.active.with(|a| a.as_deref() == Some(p.as_str())) {
                format!("row file {kc} on")
            } else {
                format!("row file {kc}")
            }
        };
        return view! {
            div(class=cls, style=pad, title=tip, on:click=move |_: MouseEvent| {
                actions::open(st, &path, 0, 0)
            }) { span(class="arr") {} span(class="ic") {} span(class="lbl") { (label) } (tg) (b) }
        };
    }
    let id = n.id.clone();
    let (i1, i2) = (id.clone(), id.clone());
    let open = move || st.expanded.with(|e| e.contains(&i1));
    let arrow = move || if open() { "\u{25be}" } else { "\u{25b8}" };
    let children = n.children;
    let body = move || {
        if !st.expanded.with(|e| e.contains(&i2)) {
            return View::default();
        }
        View::from(
            children
                .iter()
                .cloned()
                .map(|c| node(st, c, depth + 1))
                .collect::<Vec<_>>(),
        )
    };
    view! {
        div(class=format!("row dir {kc}"), style=pad, title=tip, on:click=move |_: MouseEvent| {
            st.expanded.update(|e| {
                if !e.remove(&id) {
                    e.insert(id.clone());
                }
            })
        }) { span(class="arr") { (arrow) } span(class="ic") { (mark) } span(class="lbl") { (label) } (tg) (b) }
        (body)
    }
}

pub fn explorer(st: St) -> View {
    let switch = move || {
        let names: Vec<String> = st
            .views
            .with(|v| v.iter().map(|t| t.name.clone()).collect());
        View::from(
            names
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let cls = move || if st.view.get() == i { "vw on" } else { "vw" };
                    view! { button(class=cls, on:click=move |_: MouseEvent| st.view.set(i)) { (name) } }
                })
                .collect::<Vec<_>>(),
        )
    };
    let chip = move || match st.filter.get_clone() {
        None => View::default(),
        Some(f) => view! {
            div(class="chip", title="the explorer shows only this node") {
                span { (f) }
                button(on:click=move |_: MouseEvent| st.filter.set(None)) { "show all" }
            }
        },
    };
    let tree = move || {
        let roots = st.views.with(|v| {
            v.get(st.view.get())
                .map(|t| logic::scoped(&t.roots, st.filter.get_clone().as_deref()))
                .unwrap_or_default()
        });
        View::from(
            roots
                .into_iter()
                .map(|n| node(st, n, 0))
                .collect::<Vec<_>>(),
        )
    };
    view! {
        nav(class="ex") {
            div(class="exh") { span(class="ttl") { "EXPLORER" } (switch) }
            (chip)
            div(class="tree") { (tree) }
        }
    }
}
