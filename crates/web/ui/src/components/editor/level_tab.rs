//! The Level tab: the level's file, its parent, and its `exports` (child + scope): add a child,
//! cycle a pill's scope `all -> prod -> test`, `x` removes; each change is one `add_export` or `remove_export`.

use super::op;
use crate::state::*;
use brickie_web_api::design::{ExportSpec, Op};
use sycamore::prelude::*;

/// The next scope of an export pill (`None` is `all`).
pub fn next_scope(s: Option<&str>) -> Option<String> {
    match s {
        None | Some("all") => Some("prod".into()),
        Some("prod") => Some("test".into()),
        _ => None,
    }
}

fn add(cx: &Cx, file: &str, brick: &str, scope: Option<String>) {
    let (file, brick) = (file.to_string(), brick.to_string());
    op(cx, Op::AddExport { file, brick, scope });
}

fn pill(cx: &Cx, file: &str, list: &[ExportSpec], i: usize) -> View {
    let e = &list[i];
    let label = format!("{} ({})", e.brick, e.scope.as_deref().unwrap_or("all"));
    let next = next_scope(e.scope.as_deref());
    let (c, f, b) = (cx.clone(), file.to_string(), e.brick.clone());
    let cycle = move |_: Click| add(&c, &f, &b, next.clone());
    let (c, f, b) = (cx.clone(), file.to_string(), e.brick.clone());
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
    view! { span(class="pill exp", title="click: next scope", on:click=cycle) { (label) button(class="px", on:click=remove) { "\u{00D7}" } } }
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
    let pills: Vec<View> = (0..list.len()).map(|i| pill(cx, &file, &list, i)).collect();
    let info = ch.levels.get(&lv).cloned().unwrap_or_default();
    let free: Vec<String> = info
        .bricks
        .into_iter()
        .filter(|b| !list.iter().any(|e| e.brick == *b))
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
            add(&c, &f, &b, None);
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

#[cfg(test)]
mod tests {
    #[test]
    fn export_scopes_cycle() {
        use super::next_scope;
        assert_eq!(next_scope(None).as_deref(), Some("prod"));
        assert_eq!(next_scope(Some("prod")).as_deref(), Some("test"));
        assert_eq!(next_scope(Some("test")), None);
    }
}
