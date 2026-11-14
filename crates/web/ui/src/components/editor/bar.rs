//! The header's "Design" toggle (`E`), its "Auto layout" in edit mode, and the design bar (in
//! place of the banner in edit mode): the version on screen and who made it, task chips that
//! act (planned -> select the planned bricks, no reason -> Rules tab), Undo / Redo and the last
//! error.

use super::keys::toggle_design;
use super::send;
use crate::state::*;
use brickie_web_api::design::TaskKind;
use brickie_web_api::{DesignRelayout, DesignState, RelayoutRequest};
use sycamore::prelude::*;

/// The header button: toggles design (edit) mode.
pub fn design_button(cx: &Cx) -> View {
    let on = cx.editing();
    let c = cx.clone();
    let cls = if on { "on" } else { "" };
    let tip =
        "Design (E): edit bricks and rules on the canvas; every edit is a version of brickie.toml";
    view! {
        button(id="design", class=cls, title=tip, on:click=move |_: Click| toggle_design(&c)) { "Design" }
    }
}

/// Auto layout in edit mode: forget the level's stored positions, lay it out again and fit.
pub fn relayout(cx: &Cx) {
    let level = cx.level().path.clone();
    let c = cx.clone();
    send::<DesignRelayout>(
        cx.ed,
        RelayoutRequest {
            level: level.clone(),
        },
        move |_| {
            let c = current_cx().unwrap_or(c);
            let Some(d) = c.m.design.as_deref() else {
                return;
            };
            let mut d = d.clone();
            d.layout = without_level_hints(&d.layout, &level);
            c.drag.set(Default::default());
            // No `keep`: the level is laid out anew and fitted.
            c.live.apply(d, Vec::new(), None);
        },
    );
}

fn count(ch: &DesignState, k: TaskKind) -> usize {
    ch.todo.iter().filter(|t| t.kind == k).count()
}

/// A status chip that acts when clicked.
fn chip(text: String, cls: &'static str, tip: String, act: impl Fn() + 'static) -> View {
    view! { button(class=format!("chip {cls}"), title=tip, on:click=move |_: Click| act()) { (text) } }
}

fn chips(cx: &Cx, ch: &DesignState) -> View {
    let mut out = Vec::new();
    let planned = count(ch, TaskKind::Planned);
    if planned > 0 {
        let c = cx.clone();
        let tip = "Bricks with no code yet: select them".to_string();
        out.push(chip(
            format!("{planned} planned"),
            "planned",
            tip,
            move || {
                let bricks: Vec<usize> = (0..c.m.g.bricks.len())
                    .filter(|&i| c.m.g.bricks[i].planned)
                    .collect();
                c.select(Sel::Group {
                    tags: vec![],
                    bricks,
                });
            },
        ));
    }
    let reasons = count(ch, TaskKind::Reason);
    if reasons > 0 {
        let ed = cx.ed;
        let tip = "Rules without a reason: open the Rules tab".to_string();
        out.push(chip(
            format!("{reasons} without reason"),
            "planned",
            tip,
            move || ed.tab.set("rules"),
        ));
    }
    if let Some(e) = ch.error.clone() {
        out.push(view! { span(class="chip conflict", title=e) { "design does not show" } });
    }
    View::from(out)
}

/// The version on screen: `v12 · agent`, violet when an agent made it.
fn version(cx: &Cx, ch: &DesignState) -> View {
    let ed = cx.ed;
    let text = format!("v{} \u{00B7} {}", ch.v, by_word(ch.by));
    let cls = format!("chip ver by-{}", ch.by.name());
    view! { button(class=cls, title="History of the design", on:click=move |_: Click| ed.tab.set("history")) { (text) } }
}

/// The bar of design mode.
pub fn design_bar(cx: &Cx) -> View {
    let Some(ch) = cx.m.design.clone() else {
        return View::default();
    };
    let ed = cx.ed;
    let (ver, chips, undo) = (version(cx, &ch), chips(cx, &ch), undo_redo(cx, &ch));
    view! {
        div(id="cbar") {
            span(class="muted") { "design" }
            (ver)
            (chips)
            (undo)
            span(class="err-text") { (move || ed.msg.get_clone()) }
        }
    }
}

/// Undo / Redo of the human's own versions, each naming what it would do.
fn undo_redo(cx: &Cx, ch: &DesignState) -> View {
    let (u, r) = (undo_target(&ch.history), redo_target(&ch.history));
    let (ut, rt) = (
        undo_tip("Undo", &ch.history, u),
        undo_tip("Redo", &ch.history, r),
    );
    let (c1, c2) = (cx.clone(), cx.clone());
    view! {
        span(class="seg") {
            button(id="undo", title=format!("{ut} (Ctrl+Z)"), disabled=u.is_none(), on:click=move |_: Click| super::keys::undo(&c1)) { "\u{21B6} Undo" }
            button(id="redo", title=format!("{rt} (Ctrl+Shift+Z)"), disabled=r.is_none(), on:click=move |_: Click| super::keys::redo(&c2)) { "Redo \u{21B7}" }
        }
    }
}
