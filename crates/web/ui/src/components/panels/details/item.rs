//! Group, edge and finding details.

use crate::state::*;
use std::collections::BTreeMap;
use sycamore::prelude::*;

use super::super::card::card_view;
use super::links::*;
use super::summary::sum_table;
pub(super) fn group_detail(cx: &Cx) -> View {
    let g = &cx.m.g;
    let (set, name) = cx.eval.with(|ev| (ev.sel.set(g), ev.sel.name(g)));
    let mut cnt: [BTreeMap<String, usize>; 3] = Default::default();
    for (e, x) in g.edges.iter().enumerate() {
        if x.frame() || cx.eval.with(|ev| ev.edge[e].is_none()) {
            continue;
        }
        let k = match (set.contains(&x.from), set.contains(&x.to)) {
            (false, true) => 0,
            (true, false) => 1,
            (true, true) => 2,
            _ => continue,
        };
        *cnt[k].entry(x.kind.clone()).or_default() += 1;
    }
    let by = |m: &BTreeMap<String, usize>| {
        let v: Vec<View> = m
            .iter()
            .map(|(k, n)| {
                let t = format!("{k} {n}");
                view! { span(class="tag") { (t) } }
            })
            .collect();
        if v.is_empty() {
            view! { span(class="muted") { "0" } }
        } else {
            View::from(v)
        }
    };
    let members: Vec<View> = set
        .iter()
        .map(|&b| {
            let l = brick_link(cx, b);
            view! { div { (l) } }
        })
        .collect();
    let members = View::from(members);
    let table = sum_table(cx, Some(&set));
    let (a, b, c) = (by(&cnt[0]), by(&cnt[1]), by(&cnt[2]));
    view! {
        h2 { (name) }
        p(class="muted") { "Selected group (Shift+click adds or removes, Esc clears). Only edges with an end in the group are drawn; the left list shows findings that touch it." }
        h4 { (format!("Bricks ({})", set.len())) }
        (members)
        h4 { "Into the group" } div { (a) }
        h4 { "Out of the group" } div { (b) }
        h4 { "Inside" } div { (c) }
        h4 { "Findings in scope" }
        (table)
    }
}

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
    let (from, to, kind) = (end(x.from), end(x.to), x.kind.clone());
    let title = format!("Deps findings ({})", x.diags.len());
    view! {
        h2 { (from) " " span(class="muted") { (kind) } " \u{2192} " (to) }
        div { span(class="tag") { (scope) } }
        h4 { (title) }
        (finds)
    }
}

pub(super) fn finding_detail(cx: &Cx, i: usize) -> View {
    let card = cx.m.cards[i].clone();
    let bricks: Vec<View> = card
        .d
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
    let card_v = card_view(card, cx.modal);
    view! {
        div(class="sh") { (clear) }
        p { "Bricks: " (bricks) }
        (card_v)
    }
}
