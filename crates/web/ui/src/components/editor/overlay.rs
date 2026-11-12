//! The editor's HTML overlay on the stage: the floating toolbar, the ghost of a carried brick
//! tile, the inline name box of a new or renamed brick, the delete confirmation, the tool hint,
//! the op toast and error line, the empty-level hint and the Diff panel.

use super::toolbar::{kind_name, toolbar};
use super::{op, op_then};
use crate::state::*;
use brickie_web_api::change::{BrickKind, LayoutHint};
use brickie_web_api::*;
use sycamore::prelude::*;

fn stop(e: Click) {
    e.stop_propagation();
}

/// Screen style of world point `(x, y)`.
fn at(cx: &Cx, x: f64, y: f64) -> String {
    let v = cx.view.get();
    format!("left:{}px;top:{}px", x * v.k + v.x, y * v.k + v.y)
}

/// Store the drop point (so the patched level draws the brick there), then add the brick.
fn add_draft(cx: &Cx, op: brickie_web_api::change::Op, key: String, d: Draft, name: String) {
    let hint = Some(LayoutHint { x: d.x, y: d.y });
    let c = cx.clone();
    let req = ChangeLayoutRequest { brick: key, hint };
    super::send::<ChangeLayout>(cx.ed, req, move |_| {
        op_then(&c, op, move |now, _| super::select_brick(now, &name));
    });
}

/// The inline name box of a brick being placed: Enter adds it where it was dropped.
fn draft_box(cx: &Cx, d: Draft) -> View {
    let ed = cx.ed;
    let name = create_signal(String::new());
    let members = create_signal(if d.kind == BrickKind::Module {
        "crate::".to_string()
    } else {
        String::new()
    });
    let (file, lv) = (cx.level_file().unwrap_or_default(), cx.level().path.clone());
    let c = cx.clone();
    let add = move || {
        // Read the box's signals before closing it (closing disposes them).
        let (n, m) = (name.get_clone().trim().to_string(), members.get_clone());
        if n.is_empty() {
            return;
        }
        ed.draft.set(None);
        add_draft(
            &c,
            add_brick_op(&file, d.kind, &n, &m),
            layout_key(&lv, &n),
            d,
            n,
        );
    };
    let a2 = add.clone();
    let enter = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" => a2(),
        "Escape" => ed.draft.set(None),
        _ => {}
    };
    let module = (d.kind == BrickKind::Module).then(|| view! { input(class="inmem", r#type="text", title="members: crate::module", bind:value=members) });
    let ph = format!("{} name, Enter", kind_name(d.kind).to_lowercase());
    let style = at(cx, d.x, d.y);
    view! {
        div(class="inbox draft", style=style, on:mousedown=stop, on:click=stop) {
            input(id="inline-name", r#type="text", placeholder=ph, bind:value=name, on:keydown=enter)
            (module)
        }
    }
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
        let draft = ed.draft.get().map(|d| untrack(|| draft_box(&c, d)));
        let rename = ed.rename.get_clone().map(|n| untrack(|| rename_box(&c, n)));
        let del = ed.confirm.get().then(|| untrack(|| confirm_delete(&c)));
        let diff = ed.diff.get().then(|| untrack(|| diff_panel(&c)));
        view! { (draft) (rename) (del) (diff) }
    };
    let ghost = move || match ed.carry.get() {
        Some((k, x, y)) => {
            view! { div(class="tghost", style=format!("left:{}px;top:{}px", x - 50.0, y - 18.0)) { (kind_name(k)) } }
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
