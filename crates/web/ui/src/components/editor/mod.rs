//! The design editor (edit mode, `edit=1`): the design bar, the floating toolbar over the
//! canvas, the design panel (Inspector, Rules, Level, Files, History), the canvas layer (typed
//! ports, planned edges, drop targets) and the pointer and key hooks that turn gestures into
//! ops. Every write is one version based on the version on screen; it answers an `EditReply`
//! whose levels patch the page in place: no reload, pan/zoom and selection kept.

mod aim;
mod bar;
mod clip;
mod complexity;
mod connect;
mod drop;
mod files;
mod help;
mod history;
mod inspector;
mod keys;
mod layer;
mod level_tab;
mod live;
mod marks;
mod overlay;
mod panel;
mod place;
mod retarget;
mod rules;
mod spring;
mod toolbar;
mod widgets;

pub use bar::{design_bar, design_button, relayout};
pub use drop::{cancel_drag, pointer_op, release};
pub use keys::key;
pub use layer::layer;
pub use live::start as start_live;
pub use overlay::overlay;
pub use panel::panel;

use super::call::{call, get};
use crate::state::*;
use brickie_web_api::design::Op;
use brickie_web_api::{
    Based, DesignEdit, DesignGet, EditReply, EditRequest, Endpoint, ReqOf, ResOf,
};

/// Call `E` with the editor's busy flag and error line; `then` gets the reply.
fn send<E: Endpoint>(ed: Edit, req: ReqOf<E>, then: impl FnOnce(ResOf<E>) + 'static) {
    ed.busy.set(true);
    ed.msg.set(String::new());
    call::<E>(&req, move |r| {
        ed.busy.set(false);
        match r {
            Ok(v) => then(v),
            Err(e) => ed.msg.set(e),
        }
    });
}

/// The canvas as it is now, to restore after the patch.
fn keep(cx: &Cx) -> Keep {
    let pos = positions(&cx.m.layout, &cx.drag.get_clone_untracked());
    Keep {
        level: cx.level().path.clone(),
        view: cx.view.get_untracked(),
        pos: keep_positions(&cx.m.g.bricks, &pos),
    }
}

/// Drop the brick indices the editor holds: a patched level numbers its bricks anew.
fn forget_indices(ed: Edit) {
    ed.hot.set(None);
    ed.over.set(None);
    ed.target.set(None);
}

/// Show `text` briefly.
fn toast(ed: Edit, text: String) {
    toast_with(ed, text, None);
}

/// Show `text` briefly, with an Undo button for version `undo`.
fn toast_with(ed: Edit, text: String, undo: Option<u64>) {
    ed.toast.set(text.clone());
    ed.toast_undo.set(undo);
    // A later toast keeps its own time.
    crate::platform::after(if undo.is_some() { 6000 } else { 2500 }, move || {
        if ed.toast.with_untracked(|t| *t == text) {
            ed.toast.set(String::new());
            ed.toast_undo.set(None);
        }
    });
}

/// The version on screen: the base of the next write.
fn base(cx: &Cx) -> u64 {
    cx.m.design.as_ref().map_or(0, |d| d.v)
}

/// Call a write endpoint based on the version on screen and patch the page with its reply,
/// then `then(changed)`. A `design/stale` answer (another writer was first) reloads the
/// design and sends the same request once more on the new head; when it no longer applies,
/// a one-line toast says why.
fn write<E>(cx: &Cx, mut req: ReqOf<E>, toast: String, then: impl FnOnce(&Cx, bool) + 'static)
where
    E: Endpoint<Res = EditReply>,
    ReqOf<E>: Based + Clone + 'static,
{
    req.rebase(base(cx));
    attempt::<E>(cx.clone(), req, toast, true, then);
}

fn attempt<E>(
    c: Cx,
    req: ReqOf<E>,
    text: String,
    retry: bool,
    then: impl FnOnce(&Cx, bool) + 'static,
) where
    E: Endpoint<Res = EditReply>,
    ReqOf<E>: Based + Clone + 'static,
{
    let ed = c.ed;
    ed.busy.set(true);
    ed.msg.set(String::new());
    let sent = req.clone();
    call::<E>(&sent, move |r| {
        ed.busy.set(false);
        let err = match r {
            Ok(reply) => return patch(&c, reply, text, then),
            Err(e) => e,
        };
        match on_refused(&err, retry) {
            Refusal::Retry => get::<DesignGet>(move |d| match d {
                Ok(d) => {
                    let mut req = req;
                    req.rebase(d.v);
                    attempt::<E>(c, req, text, false, then);
                }
                Err(e) => ed.msg.set(e),
            }),
            Refusal::Show(why) => {
                self::toast(ed, why);
                // The page may be behind another writer: catch up.
                live::refresh_soon();
            }
        }
    });
}

/// Patch the page with a write's reply, then `then(changed)`.
fn patch(c: &Cx, reply: EditReply, text: String, then: impl FnOnce(&Cx, bool)) {
    let EditReply {
        changed,
        design,
        levels,
    } = reply;
    // A gesture's earlier write may have re-rendered the level since this call: patch the
    // level on screen now.
    let c = current_cx().unwrap_or_else(|| c.clone());
    let text = match changed {
        true => format!("v{} \u{00B7} {text}", design.v),
        false => format!("{text}: no change"),
    };
    self::toast(c.ed, text);
    // Leave fields before the re-render removes them (their blur handlers are still alive).
    crate::platform::blur_active();
    forget_indices(c.ed);
    let broken = design.error.clone();
    c.live.apply(design, levels, Some(keep(&c)));
    if let Some(e) = broken {
        c.ed.msg.set(format!(
            "The design no longer shows: {e} \u{00B7} Ctrl+Z undoes"
        ));
    }
    // The patch re-rendered the page: act on the new level state.
    if let Some(now) = current_cx() {
        then(&now, changed);
    }
}

/// Apply ops as one version (toast: what they do).
fn op(cx: &Cx, op: Op) {
    ops_then(cx, vec![op], |_, _| {});
}

/// Apply several ops as one version.
fn ops(cx: &Cx, ops: Vec<Op>) {
    ops_then(cx, ops, |_, _| {});
}

/// Select brick `name` of the level on screen.
fn select_brick(cx: &Cx, name: &str) {
    cx.go(cx.params.with(|p| p.with(|p| p.brick = name.to_string())));
}

/// Rename brick `from` of `file` to `to` (when it changes) and keep it selected.
fn rename(cx: &Cx, file: &str, from: &str, to: &str) {
    let Some(o) = rename_op(file, from, to) else {
        return;
    };
    let to = to.trim().to_string();
    ops_then(cx, vec![o], move |now, _| select_brick(now, &to));
}

/// Apply ops as one version, then `then` with the new state.
fn ops_then(cx: &Cx, ops: Vec<Op>, then: impl FnOnce(&Cx, bool) + 'static) {
    let text = match ops.as_slice() {
        [one] => describe(one),
        [first, rest @ ..] => format!("{} (+{} more)", describe(first), rest.len()),
        [] => return,
    };
    write::<DesignEdit>(cx, EditRequest { base: 0, ops }, text, then);
}
