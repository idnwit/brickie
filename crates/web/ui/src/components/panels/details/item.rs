//! Edge and finding details.

use crate::state::*;
use sycamore::prelude::*;

use super::super::card::card_view;
use super::links::*;

pub(super) fn edge_detail(cx: &Cx, e: usize) -> View {
    let g = &cx.m.g;
    let x = g.edges[e].clone();
    let finds: Vec<View> = x
        .diags
        .iter()
        .cloned()
        .map(|id| {
            let href = cx.params.get_clone().with(|p| p.item = id.clone()).url();
            let (c, id2) = (cx.clone(), id.clone());
            view! { div { a(href=href, class="mono", on:click=move |ev: Click| {
                ev.prevent_default();
                c.go(c.params.get_clone().with(|p| p.item = id2.clone()));
            }) { (id) } } }
        })
        .collect();
    let finds = View::from(finds);
    let scope = if x.test { "test scope only" } else { "prod" };
    let end = |b: usize| {
        if x.frame() && !g.bricks[b].ghost {
            let n = format!("{} (frame)", cx.level().name());
            view! { span { (n) } }
        } else {
            brick_link(cx, b)
        }
    };
    // Brick paths of the ends; the frame end is the current level's composite.
    let path = |b: usize| {
        if x.frame() && !g.bricks[b].ghost {
            cx.level().path.clone()
        } else {
            g.bricks[b].path.clone()
        }
    };
    let sites = super::iface::edge_section(cx, path(x.from), path(x.to), x.kind.clone());
    let (from, to, kind) = (end(x.from), end(x.to), x.kind.clone());
    let title = format!("Deps findings ({})", x.diags.len());
    view! {
        h2 { (from) " " span(class="muted") { (kind) } " \u{2192} " (to) }
        div { span(class="tag") { (scope) } }
        h4 { (title) }
        (finds)
        (sites)
    }
}

pub(super) fn finding_detail(cx: &Cx, i: usize) -> View {
    let d = cx.m.diags[i].clone();
    let bricks: Vec<View> = d
        .bricks
        .iter()
        .filter_map(|n| cx.brick(n))
        .map(|b| {
            let l = brick_link(cx, b);
            view! { span { (l) " " } }
        })
        .collect();
    let bricks = View::from(bricks);
    let c = cx.clone();
    let clear = view! { a(href="#", on:click=move |e: Click| { e.prevent_default(); c.go(c.params.get_clone().with(|p| p.item.clear())); }) { "\u{2190} back" } };
    let card_v = card_view(d, cx.modal);
    view! {
        div(class="sh") { (clear) }
        p { "Bricks: " (bricks) }
        (card_v)
    }
}
