//! Links between bricks and edges.

use crate::state::*;
use sycamore::prelude::*;

pub(super) fn brick_link(cx: &Cx, b: usize) -> View {
    let name = cx.m.g.bricks[b].name.clone();
    let href = Sel::Brick(b).apply(&cx.params.get_clone(), &cx.m.g).url();
    let c = cx.clone();
    view! { a(href=href, on:click=move |e: Click| { e.prevent_default(); c.select(Sel::Brick(b)); }) { (name) } }
}

pub(super) fn edge_link(cx: &Cx, e: usize, other: usize) -> View {
    let x = cx.m.g.edges[e].clone();
    let href = Sel::Edge(e).apply(&cx.params.get_clone(), &cx.m.g).url();
    let c = cx.clone();
    let label = cx.m.g.bricks[other].name.clone();
    let tag = format!("tag{}", if x.diags.is_empty() { "" } else { " debt" });
    let note = if x.test { " (tests only)" } else { "" };
    view! {
        div { a(href=href, on:click=move |ev: Click| { ev.prevent_default(); c.select(Sel::Edge(e)); }) {
            span(class=tag) { (x.kind.clone()) } (label)
        } span(class="muted") { (note) } }
    }
}
