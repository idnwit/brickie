//! The stage overlay of the editor: the hint of the armed tool or link, the name form of a
//! brick being placed, and the confirmations of a dropped link (reason) or move.

use super::palette::kind_name;
use super::{send, send_reload};
use crate::state::*;
use brickie_web_api::change::{BrickKind, LayoutHint};
use brickie_web_api::*;
use sycamore::prelude::*;

fn place(cx: &Cx, d: Draft) -> View {
    let ed = cx.ed;
    let name = create_signal(String::new());
    let members = create_signal(String::new());
    let v = cx.view.get();
    let style = format!("left:{}px;top:{}px", d.x * v.k + v.x, d.y * v.k + v.y);
    let (file, lv) = (cx.level_file().unwrap_or_default(), cx.level().path.clone());
    let add = move |_: Click| {
        let n = name.get_clone().trim().to_string();
        if n.is_empty() {
            return;
        }
        let op = add_brick_op(&file, d.kind, &n, &members.get_clone());
        let hint = ChangeLayoutRequest {
            brick: layout_key(&lv, &n),
            hint: Some(LayoutHint { x: d.x, y: d.y }),
        };
        send::<ChangeOp>(ed, ChangeOpRequest { op, note: None }, move |_| {
            send_reload::<ChangeLayout>(ed, hint);
        });
    };
    let module = (d.kind == BrickKind::Module).then(
        || view! { input(r#type="text", placeholder="members: crate::module", bind:value=members) },
    );
    let title = format!("New {}", kind_name(d.kind).to_lowercase());
    view! {
        div(class="edpop", style=style, on:mousedown=|e: Click| e.stop_propagation(), on:click=|e: Click| e.stop_propagation()) {
            b { (title) }
            input(id="draft-name", r#type="text", placeholder="name", bind:value=name)
            (module)
            div(class="row") {
                button(on:click=move |_: Click| ed.draft.set(None)) { "Cancel" }
                button(class="go", on:click=add) { "Add" }
            }
        }
    }
}

fn confirm(cx: &Cx, p: Pending) -> View {
    let ed = cx.ed;
    let reason = create_signal(String::new());
    let (file, lv) = (cx.level_file().unwrap_or_default(), cx.level().path.clone());
    let (text, needs_reason) = match &p {
        Pending::Rule { pick, to } => (
            format!(
                "{} {} {} \u{2192} {to} ({})",
                pick.action, pick.from, pick.kind, pick.scope
            ),
            true,
        ),
        Pending::Move { from, into } => (format!("Move {from} into {into}?"), false),
    };
    let ok = move |_: Click| match &p {
        Pending::Rule { pick, to } => {
            let op = rule_op(&file, pick, to, &reason.get_clone());
            send_reload::<ChangeOp>(ed, ChangeOpRequest { op, note: None });
        }
        Pending::Move { from, into } => {
            let req = ChangeMoveRequest {
                from: brick_path(&lv, from),
                into: brick_path(&lv, into),
                note: None,
            };
            send_reload::<ChangeMove>(ed, req);
        }
    };
    let why = needs_reason.then(|| view! { input(id="rule-reason", r#type="text", placeholder="reason (why this link)", bind:value=reason) });
    view! {
        div(class="edpop center", on:mousedown=|e: Click| e.stop_propagation(), on:click=|e: Click| e.stop_propagation()) {
            b { (text) }
            (why)
            div(class="row") {
                button(on:click=move |_: Click| { ed.pending.set(None); ed.link.set(None); }) { "Cancel" }
                button(class="go", on:click=ok) { "OK" }
            }
        }
    }
}

/// The overlay inside `#stage` (empty outside edit mode).
pub fn overlay(cx: &Cx) -> View {
    if !cx.editing() {
        return View::default();
    }
    let ed = cx.ed;
    let hint = move || {
        if let Some(l) = ed.link.get_clone() {
            format!(
                "Drop on a sibling: {} {} \u{2192} ? ({}) \u{00B7} Esc to cancel",
                l.action, l.kind, l.scope
            )
        } else if let Some(k) = ed.tool.get() {
            format!(
                "Click on the canvas to place a {}",
                kind_name(k).to_lowercase()
            )
        } else {
            String::new()
        }
    };
    let (c1, c2) = (cx.clone(), cx.clone());
    let draft = move || {
        ed.draft
            .get()
            .map(|d| untrack(|| place(&c1, d)))
            .unwrap_or_default()
    };
    let pending = move || {
        ed.pending
            .get_clone()
            .map(|p| untrack(|| confirm(&c2, p)))
            .unwrap_or_default()
    };
    let msg = move || ed.msg.get_clone();
    view! {
        div(class="edhint", style=move || if hint().is_empty() { "display:none" } else { "" }) { (hint) }
        (draft)
        (pending)
        div(class="edmsg err-text") { (msg) }
    }
}
