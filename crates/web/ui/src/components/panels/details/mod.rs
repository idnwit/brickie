//! The right panel (`renderDetail`): overview, brick, edge or finding details, then the
//! scoped functions table and the legend.

mod brick;
mod iface;
mod item;
mod links;
mod summary;
mod tabs;

use super::{functions::functions_view, legend::legend};
use crate::state::*;
use brick::brick_detail;
use item::{edge_detail, finding_detail};
use summary::summary;
use sycamore::prelude::*;
use tabs::{DetailTab, active_tab, brick_target, embedded_doc, tab_bar};

fn overview_content(cx: &Cx) -> View {
    let item = cx.params.with(|p| p.item.clone());
    if let Some(i) =
        cx.m.diags
            .iter()
            .position(|d| d.id == item)
            .filter(|_| !item.is_empty())
    {
        return finding_detail(cx, i);
    }
    match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::None => summary(cx),
        Sel::Brick(b) => brick_detail(cx, b),
        Sel::Member(b, m) => brick::member_detail(cx, b, m),
        Sel::Edge(e) => edge_detail(cx, e),
    }
}

/// `renderDetail`.
pub fn detail(cx: &Cx) -> View {
    let tab = create_signal((String::new(), DetailTab::Overview));
    let target = {
        let cx = cx.clone();
        create_memo(move || brick_target(&cx))
    };
    let doc_open = move || {
        target.with(|t| {
            t.as_ref()
                .is_some_and(|t| active_tab(tab, t) != DetailTab::Overview)
        })
    };
    let aside_cls = move || if doc_open() { "docopen" } else { "" };
    let header_tabs = move || target.get_clone().map(|t| tab_bar(tab, &t));
    let body = {
        let cx = cx.clone();
        move || {
            if let Some(t) = target.get_clone() {
                let active = active_tab(tab, &t);
                if active != DetailTab::Overview {
                    return embedded_doc(&cx, &t, active);
                }
            }
            let content = overview_content(&cx);
            let fns = if cx.eval.with(|ev| matches!(ev.sel, Sel::Edge(_))) {
                View::default()
            } else {
                functions_view(&cx)
            };
            view! { (content) (fns) }
        }
    };
    let leg = {
        let cx = cx.clone();
        move || {
            if doc_open() {
                View::default()
            } else {
                legend(&cx)
            }
        }
    };
    view! {
        aside(id="detail", class=aside_cls) {
            (header_tabs)
            div(id="dcont") { (body) }
            (leg)
        }
    }
}
