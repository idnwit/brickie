//! The Level tab: the level's file, its parent, and its `exports`: add a child, `x` removes;
//! each change is one `add_export` or `remove_export`.

use super::op;
use crate::state::*;
use brickie_web_api::design::Op;
use sycamore::prelude::*;

fn add(cx: &Cx, file: &str, brick: &str) {
    let (file, brick) = (file.to_string(), brick.to_string());
    op(cx, Op::AddExport { file, brick });
}

fn pill(cx: &Cx, file: &str, name: &str) -> View {
    let label = name.to_string();
    let (c, f, b) = (cx.clone(), file.to_string(), name.to_string());
    let remove = move |e: Click| {
        e.stop_propagation();
        op(
            &c,
            Op::RemoveExport {
                file: f.clone(),
                brick: b.clone(),
            },
        );
    };
    view! { span(class="pill exp") { (label) button(class="px", on:click=remove) { "\u{00D7}" } } }
}

pub fn level_tab(cx: &Cx) -> View {
    let (Some(ch), Some(file)) = (cx.m.design.clone(), cx.level_file()) else {
        return View::default();
    };
    let lv = cx.level().path.clone();
    let list = ch
        .sheets
        .get(&file)
        .map(|s| s.exports.clone())
        .unwrap_or_default();
    let pills: Vec<View> = list.iter().map(|e| pill(cx, &file, e)).collect();
    let info = ch.levels.get(&lv).cloned().unwrap_or_default();
    let free: Vec<String> = info
        .bricks
        .into_iter()
        .filter(|b| !list.contains(b))
        .collect();
    let pick = create_signal(String::new());
    let opts = View::from(
        free.iter()
            .map(|b| {
                let (v, t) = (b.clone(), b.clone());
                view! { option(value=v) { (t) } }
            })
            .collect::<Vec<_>>(),
    );
    let (c, f) = (cx.clone(), file.clone());
    let add_one = move |_: Click| {
        let b = pick.get_clone();
        if !b.is_empty() {
            add(&c, &f, &b);
        }
    };
    let parent = parent_level(&lv).map(|p| {
        let (c, target) = (cx.clone(), p.clone());
        view! { p { "parent: " a(href="#", on:click=move |e: Click| { e.prevent_default(); c.go(c.params.with(|p| p.at_level(&target))); }) { (p) } } }
    });
    let pills = View::from(pills);
    let root = (lv == "root").then(|| view! { p(class="muted") { "The root level exports nothing (exports apply to composites)." } });
    view! {
        h3 { (lv) }
        p(class="muted mono") { (file) }
        (parent)
        (root)
        label { "exports (children visible from outside)" }
        div(class="pills") { (pills) }
        div(class="row") { select(bind:value=pick) { option(value="") { "child\u{2026}" } (opts) } button(on:click=add_one) { "+ export" } }
    }
}
