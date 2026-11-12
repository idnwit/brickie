//! The design editor (edit mode, `change=1` with a change open): the change bar and its
//! dialogs, the floating toolbar over the canvas, the design panel (Inspector, Rules, Level,
//! Files, Ops), the canvas layer (typed ports, planned edges, agent badges, drop targets) and
//! the pointer and key hooks that turn gestures into ops. Every write answers a `ChangeReply`
//! whose levels patch the page in place: no reload, pan/zoom and selection kept.

mod aim;
mod bar;
mod clip;
mod complexity;
mod connect;
mod dialogs;
mod drop;
mod files;
mod help;
mod inspector;
mod keys;
mod layer;
mod level_tab;
mod live;
mod log;
mod marks;
mod overlay;
mod panel;
mod place;
mod retarget;
mod rules;
mod spring;
mod toolbar;
mod widgets;

pub use bar::{change_bar, design_button};
pub use drop::{cancel_drag, pointer_op, release};
pub use keys::key;
pub use layer::layer;
pub use live::start as start_live;
pub use overlay::overlay;
pub use panel::panel;

use super::call::call;
use crate::state::*;
use brickie_web_api::change::Op;
use brickie_web_api::{ChangeOp, ChangeOpRequest, ChangeReply, Endpoint, ReqOf, ResOf};

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
    ed.toast.set(text.clone());
    // A later toast keeps its own time.
    crate::platform::after(2500, move || {
        if ed.toast.with_untracked(|t| *t == text) {
            ed.toast.set(String::new());
        }
    });
}

/// Call a write endpoint answering a `ChangeReply`, patch the page, then `then(done)`.
fn write<E, T>(cx: &Cx, req: ReqOf<E>, toast: String, then: impl FnOnce(&Cx, T) + 'static)
where
    E: Endpoint<Res = ChangeReply<T>>,
    T: 'static,
{
    let c = cx.clone();
    send::<E>(cx.ed, req, move |reply| {
        let ChangeReply {
            done,
            change,
            levels,
        } = reply;
        // A gesture's earlier op may have re-rendered the level since this call: patch the
        // level on screen now.
        let c = current_cx().unwrap_or(c);
        self::toast(c.ed, toast);
        // Leave fields before the re-render removes them (their blur handlers are still alive).
        crate::platform::blur_active();
        forget_indices(c.ed);
        let broken = change.error.clone();
        c.live.apply(change, levels, Some(keep(&c)));
        if let Some(e) = broken {
            // The op is recorded but the design no longer checks: say why, where it happened.
            c.ed.msg.set(format!(
                "Recorded, but the design no longer checks (canvas not updated): {e} \u{00B7} Ctrl+Z undoes"
            ));
        }
        // The patch re-rendered the page: act on the new level state.
        if let Some(now) = current_cx() {
            then(&now, done);
        }
    });
}

/// Record a human op (toast: its name).
fn op(cx: &Cx, op: Op) {
    op_then(cx, op, |_, _| {});
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
    op_then(cx, o, move |now, _| select_brick(now, &to));
}

/// Record a human op, then `then` with the new state.
fn op_then(cx: &Cx, op: Op, then: impl FnOnce(&Cx, brickie_web_api::change::OpRecord) + 'static) {
    let toast = format!("{} \u{00B7} {}", op.name(), op.file());
    write::<ChangeOp, _>(cx, ChangeOpRequest { op, note: None }, toast, then);
}
