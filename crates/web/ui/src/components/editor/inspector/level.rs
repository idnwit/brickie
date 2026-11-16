//! The inspector with nothing selected: the level's file, its bricks and
//! tags, and how to start.

use crate::state::*;
use sycamore::prelude::*;

pub fn level_view(cx: &Cx) -> View {
    let lv = cx.level().clone();
    let ch = cx.m.design.clone();
    let file = cx.level_file().unwrap_or_else(|| lv.file.clone());
    let sheet = ch
        .as_deref()
        .and_then(|c| c.sheets.get(&file))
        .cloned()
        .unwrap_or_default();
    let info = ch
        .as_deref()
        .and_then(|c| c.levels.get(&lv.path))
        .cloned()
        .unwrap_or_default();
    let counts = format!(
        "{} bricks \u{00B7} {} rules in this file",
        info.bricks.len(),
        sheet.rules.len()
    );
    let tags: Vec<View> = info
        .tags
        .iter()
        .map(|t| {
            let t = t.clone();
            view! { span(class="pill tag") { (t) } }
        })
        .collect();
    let ed = cx.ed;
    let tags = View::from(tags);
    view! {
        h2 { (lv.name().to_string()) }
        p(class="muted mono") { (file) }
        p(class="muted") { (counts) " " a(href="#", on:click=move |e: Click| { e.prevent_default(); ed.tab.set("rules"); }) { "edit" } }
        label { "tags" }
        div(class="pills") { (tags) }
        p(class="muted") { "Nothing selected. Drag a brick tile from the toolbar onto the canvas; hover a brick and drag one of its typed ports onto a sibling to add a rule; drop a brick on another to move it inside (rest on a group to open it), or Ctrl+X / Ctrl+V across levels; select a rule and drag its arrowhead to retarget it. Press ? for all shortcuts." }
    }
}
