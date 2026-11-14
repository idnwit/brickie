//! The History tab: the design's versions, newest first, each with its author (agent violet,
//! you, hand edit), age, what it did (its diff lines) and an Undo that reverts it as a new
//! version.

use super::keys::undo_version;
use crate::state::*;
use brickie_web_api::design::Version;
use sycamore::prelude::*;

fn row(cx: &Cx, v: &Version, head: u64) -> View {
    let (c, n) = (cx.clone(), v.v);
    let title = format!("v{} \u{00B7} {}", v.v, by_word(v.by));
    let what = version_words(v);
    let lines: Vec<View> = (v.diff.iter().take(8))
        .map(|d| {
            let d = d.clone();
            view! { li(class="mono") { (d) } }
        })
        .collect();
    let lines = View::from(lines);
    let cls = format!("hver by-{}", v.by.name());
    let undo = (!v.full).then(|| {
        let tip = format!("Revert v{n} as a new version");
        view! { button(class="mini", title=tip, on:click=move |_: Click| undo_version(&c, n)) { "Undo" } }
    });
    let here = (v.v == head).then(|| view! { span(class="muted") { " \u{00B7} head" } });
    view! {
        div(class=cls) {
            div(class="row") { b { (title) } (here) span(class="grow") {} (undo) }
            div { (what) }
            ul(class="hdiff") { (lines) }
        }
    }
}

pub fn history(cx: &Cx) -> View {
    let Some(d) = cx.m.design.clone() else {
        return View::default();
    };
    let rows: Vec<View> = d.history.iter().rev().map(|v| row(cx, v, d.v)).collect();
    let rows = View::from(rows);
    view! {
        p(class="muted") { "Every edit is a version of the brickie.toml files. Undo reverts one as a new version." }
        (rows)
    }
}
