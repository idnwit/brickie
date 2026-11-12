//! The design editor (edit mode, `change=1` with a change open): the change bar and its
//! dialogs, the palette and inspector panel, the ops log, and the stage overlay and pointer
//! hooks that turn drags into ops. Every op goes to the server's change endpoints; the page
//! then reloads, so the canvas always shows the server's preview.

mod bar;
mod dialogs;
mod drop;
mod inspector;
mod log;
mod overlay;
mod palette;

pub use bar::{change_bar, design_button};
pub use drop::release;
pub use overlay::overlay;
pub use palette::panel;

use super::call::call;
use crate::platform;
use crate::state::Edit;
use brickie_web_api::{Endpoint, ReqOf, ResOf};

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

/// Call `E`, then reload the page (the canvas shows the new preview).
fn send_reload<E: Endpoint>(ed: Edit, req: ReqOf<E>) {
    send::<E>(ed, req, |_| platform::reload());
}
