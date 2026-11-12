//! The editor's HTML overlay on the stage: the floating toolbar, the ghost of a carried brick
//! tile, the inline name box of a new or renamed brick, the delete confirmation, the tool hint,
//! the op toast and error line, the empty-level hint and the Diff panel.

use super::op;
use super::place::{draft_box, lane_box};
use super::toolbar::{kind_name, toolbar};
use crate::state::*;

use sycamore::prelude::*;

pub(super) fn stop(e: Click) {
    e.stop_propagation();
}

/// Screen style of world point `(x, y)`.
pub(super) fn at(cx: &Cx, x: f64, y: f64) -> String {
    let v = cx.view.get();
    format!("left:{}px;top:{}px", x * v.k + v.x, y * v.k + v.y)
}

/// Inline rename on the box (F2): Enter renames.
fn rename_box(cx: &Cx, from: String) -> View {
    let ed = cx.ed;
    let Some(b) = cx.brick(&from) else {
        return View::default();
    };
    let r = positions(&cx.m.layout, &cx.drag.get_clone_untracked())[b];
    let name = create_signal(from.clone());
    let (c, file) = (cx.clone(), cx.level_file().unwrap_or_default());
    let enter = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            let to = name.get_clone();
            ed.rename.set(None);
            super::rename(&c, &file, &from, &to);
        }
        "Escape" => ed.rename.set(None),
        _ => {}
    };
    let style = at(cx, r.x, r.y);
    view! {
        div(class="inbox", style=style, on:mousedown=stop, on:click=stop) {
            input(id="inline-name", r#type="text", bind:value=name, on:keydown=enter)
        }
    }
}

/// The delete confirmation of the selected brick, listing what refers to it.
fn confirm_delete(cx: &Cx) -> View {
    let ed = cx.ed;
    let Sel::Brick(b) = cx.eval.with(|ev| ev.sel.clone()) else {
        return View::default();
    };
    let name = cx.m.g.bricks[b].name.clone();
    let refs =
        cx.m.change
            .as_deref()
            .map(|ch| references(ch, &cx.level().path, &name))
            .unwrap_or_default();
    let items: Vec<View> = refs.into_iter().map(|r| view! { li { (r) } }).collect();
    let (c, file, n2) = (
        cx.clone(),
        cx.level_file().unwrap_or_default(),
        name.clone(),
    );
    let yes = move |_: Click| {
        ed.confirm.set(false);
        c.select(Sel::None);
        op(
            &c,
            brickie_web_api::change::Op::RemoveBrick {
                file: file.clone(),
                name: n2.clone(),
            },
        );
    };
    let (items, title) = (View::from(items), format!("Delete {name}?"));
    view! {
        div(class="edpop center", on:mousedown=stop, on:click=stop) {
            b { (title) }
            ul(class="refs") { (items) }
            div(class="row") {
                button(on:click=move |_: Click| ed.confirm.set(false)) { "Cancel" }
                button(id="confirm-delete", class="danger", on:click=yes) { "Delete" }
            }
        }
    }
}

fn hint(ed: Edit) -> String {
    if let Some(c) = ed.connect.get_clone() {
        return format!(
            "{} {} \u{2192} ? ({}): drop on a sibling or a tag lane \u{00B7} Esc cancels",
            c.from,
            c.ty.name(),
            ed.scope.get()
        );
    }
    if let Some(t) = ed.link_type.get() {
        return format!("Drag a {} port from a brick", t.name());
    }
    match ed.brick_tool.get() {
        Some(k) => format!(
            "Click on the canvas to place a {} (Tab: next kind)",
            kind_name(k).to_lowercase()
        ),
        None => String::new(),
    }
}

fn diff_panel(cx: &Cx) -> View {
    let Some(ch) = cx.m.change.clone() else {
        return View::default();
    };
    let rows: Vec<View> = ch.diff.iter().map(|d| {
        let t = format!("{} \u{00B7} {}", d.file, d.key);
        let (b, a) = (d.before.clone().unwrap_or_default(), d.after.clone().unwrap_or_default());
        view! { div(class="drow") { b(class="mono") { (t) } div(class="dcols") { pre(class="before") { (b) } pre(class="after") { (a) } } } }
    }).collect();
    let rows = View::from(rows);
    view! { div(class="diffp", on:mousedown=stop, on:click=stop, on:wheel=|e: web_sys::WheelEvent| e.stop_propagation()) { h4 { "Diff (before | after)" } (rows) } }
}

/// The overlay inside `#stage` (empty outside edit mode).
pub fn overlay(cx: &Cx) -> View {
    if !cx.editing() {
        return View::default();
    }
    let ed = cx.ed;
    let c = cx.clone();
    let floating = move || {
        let c = c.clone();
        let draft = ed.draft.get_clone().map(|d| untrack(|| draft_box(&c, d)));
        let lane = ed
            .lane_ask
            .get_clone()
            .map(|(n, x, y)| untrack(|| lane_box(&c, n, x, y)));
        let rename = ed.rename.get_clone().map(|n| untrack(|| rename_box(&c, n)));
        let del = ed.confirm.get().then(|| untrack(|| confirm_delete(&c)));
        let diff = ed.diff.get().then(|| untrack(|| diff_panel(&c)));
        view! { (draft) (lane) (rename) (del) (diff) }
    };
    let ghost = move || match ed.carry.get() {
        Some((k, x, y)) => {
            let no = ed.target.with(|t| matches!(t, Some(MoveTarget::No(_))));
            let cls = if no { "tghost no" } else { "tghost" };
            view! {
                div(class=cls, style=format!("left:{}px;top:{}px", x - 50.0, y - 18.0)) {
                    (kind_name(k))
                    span(class="aimlbl") { (ed.aim_label.get_clone()) }
                }
            }
        }
        None => View::default(),
    };
    let empty = cx.m.g.bricks.iter().all(|b| b.ghost).then(|| view! { div(class="emptyhint") { "Drag a Crate, Module or Group here from the toolbar" } });
    let tb = toolbar(cx);
    view! {
        (tb)
        (empty)
        (floating)
        (ghost)
        div(class="edhint", style=move || if hint(ed).is_empty() { "display:none" } else { "" }) { (move || hint(ed)) }
        div(class="toast", style=move || if ed.toast.with(String::is_empty) { "display:none" } else { "" }) { (move || ed.toast.get_clone()) }
        div(class="edmsg err-text") { (move || ed.msg.get_clone()) }
    }
}
