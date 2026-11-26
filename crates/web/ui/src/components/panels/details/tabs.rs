//! Overview / README.md / Docs tab strip and embedded doc/README frame for the right detail panel.

use crate::state::*;
use brickie_web_api::{docs_href, readme_href};
use sycamore::prelude::*;

/// Active tab of the detail panel when viewing the current level or a selected brick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetailTab {
    Overview,
    Readme,
    Docs,
}

/// The brick currently targeted by the detail panel tabs (current level, selected brick, or
/// selected peer-group member).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BrickTarget {
    pub(super) path: String,
    pub(super) has_readme: bool,
}

/// The brick whose Overview / README.md / Docs tabs should be shown, or `None` for edges,
/// findings, and local peer-group containers.
pub(super) fn brick_target(cx: &Cx) -> Option<BrickTarget> {
    let item = cx.params.with(|p| p.item.clone());
    if !item.is_empty() && cx.m.diags.iter().any(|d| d.id == item) {
        return None;
    }
    match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::None => {
            let lv = cx.level();
            Some(BrickTarget {
                path: lv.path.clone(),
                has_readme: lv.has_readme,
            })
        }
        Sel::Brick(b) => {
            let br = &cx.m.g.bricks[b];
            if br.is_group && !br.ghost {
                None
            } else {
                Some(BrickTarget {
                    path: br.path.clone(),
                    has_readme: br.has_readme,
                })
            }
        }
        Sel::Member(b, m) => {
            let mem = &cx.m.g.bricks[b].group_members[m];
            Some(BrickTarget {
                path: mem.path.clone(),
                has_readme: mem.has_readme,
            })
        }
        Sel::Edge(_) => None,
    }
}

/// The active tab for `target`, falling back to `Overview` when the stored tab belongs to a
/// different brick or points at a missing README.
pub(super) fn active_tab(tab: Signal<(String, DetailTab)>, target: &BrickTarget) -> DetailTab {
    tab.with(|(p, t)| {
        if *p == target.path && (*t != DetailTab::Readme || target.has_readme) {
            *t
        } else {
            DetailTab::Overview
        }
    })
}

fn tab_button(
    tab: Signal<(String, DetailTab)>,
    target: &BrickTarget,
    which: DetailTab,
    label: &'static str,
) -> View {
    let t_cls = target.clone();
    let cls = move || {
        if active_tab(tab, &t_cls) == which {
            "on"
        } else {
            ""
        }
    };
    let path = target.path.clone();
    view! {
        button(class=cls, on:click=move |_: Click| tab.set((path.clone(), which))) { (label) }
    }
}

/// Tab bar: Overview, README.md (when present), Docs, plus a quick link to open Docs in a new tab.
pub(super) fn tab_bar(tab: Signal<(String, DetailTab)>, target: &BrickTarget) -> View {
    let overview = tab_button(tab, target, DetailTab::Overview, "Overview");
    let readme = target
        .has_readme
        .then(|| tab_button(tab, target, DetailTab::Readme, "README.md"));
    let docs = tab_button(tab, target, DetailTab::Docs, "Docs");
    let docs_url = docs_href(&target.path);
    view! {
        div(class="dtabs") {
            (overview)
            (readme)
            (docs)
            a(class="dtab-ext", href=docs_url, target="_blank", title="Open docs in a new tab") {
                "Docs \u{2197}"
            }
        }
    }
}

/// Embedded README.md or Docs iframe with Full-screen modal expansion and open-in-new-tab link.
pub(super) fn embedded_doc(cx: &Cx, target: &BrickTarget, kind: DetailTab) -> View {
    let (title, full_url, new_tab_label) = match kind {
        DetailTab::Readme => ("README.md", readme_href(&target.path), "Open \u{2197}"),
        DetailTab::Docs => ("Docs", docs_href(&target.path), "Open docs \u{2197}"),
        DetailTab::Overview => return View::default(),
    };
    let embed_url = format!("{full_url}?embed=1");
    let modal = cx.modal;
    let open_full_url = full_url.clone();
    view! {
        div(class="docbar") {
            span(class="muted") { (title) }
            span(class="docbar-acts") {
                button(
                    title="Expand full view",
                    on:click=move |_: Click| modal.set(Some(open_full_url.clone()))
                ) { "\u{26F6} Full" }
                a(href=full_url, target="_blank", title="Open in a new tab") { (new_tab_label) }
            }
        }
        iframe(class="docframe", src=embed_url, title=title)
    }
}
