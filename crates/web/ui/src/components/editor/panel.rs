//! The design panel (right column in edit mode): Inspector, Rules, Level, Files and Ops tabs,
//! all scoped to the level on screen.

use super::{complexity, files, inspector, level_tab, log, rules};
use crate::state::*;
use sycamore::prelude::*;

const TABS: [(&str, &str); 5] = [
    ("inspect", "Inspector"),
    ("rules", "Rules"),
    ("level", "Level"),
    ("files", "Files"),
    ("ops", "Ops"),
];

fn tabs(cx: &Cx) -> View {
    let ed = cx.ed;
    let proposed =
        cx.m.change
            .as_ref()
            .map_or(0, |c| c.log.iter().filter(|r| pending(r)).count());
    let items: Vec<View> = TABS
        .iter()
        .map(|&(id, name)| {
            let label = match (id, proposed) {
                ("ops", n) if n > 0 => format!("{name} ({n})"),
                _ => name.to_string(),
            };
            let cls = move || format!("tab{}", if ed.tab.get() == id { " on" } else { "" });
            view! { button(class=cls, "data-tab"=id, on:click=move |_: Click| ed.tab.set(id)) { (label) } }
        })
        .collect();
    let items = View::from(items);
    view! { div(class="tabs") { (items) } }
}

/// The right column in edit mode.
pub fn panel(cx: &Cx) -> View {
    let ed = cx.ed;
    // Selecting a brick or a measured edge ends a rule selection.
    let params = cx.params;
    create_effect(move || {
        let any = params.with(|p| !p.brick.is_empty() || !p.edge.is_empty());
        if any {
            ed.rule.set(None);
        }
    });
    let tabs = tabs(cx);
    let c = cx.clone();
    let body = move || match ed.tab.get() {
        "rules" => untrack(|| {
            let r = rules::rules(&c);
            let x = complexity::complexity(&c);
            view! { (r) (x) }
        }),
        "level" => untrack(|| level_tab::level_tab(&c)),
        "files" => untrack(|| files::files(&c)),
        "ops" => log::log(&c),
        _ => inspector::inspector(&c),
    };
    view! {
        aside(id="detail", class="editor") {
            (tabs)
            div(id="dcont") { (body) }
        }
    }
}
