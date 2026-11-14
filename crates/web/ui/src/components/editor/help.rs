//! The keyboard and gesture sheet of the editor (`?`, or the toolbar's `?` button): every
//! shortcut in one place, so they can be found without reading the docs.

use super::overlay::stop;
use crate::state::*;
use sycamore::prelude::*;

const KEYS: [(&str, &str); 16] = [
    ("V / H", "select tool / hand (pan) tool"),
    ("B, Tab", "brick tool, next kind (crate, module, group)"),
    ("L", "link tool: ports on every brick"),
    ("1 2 3", "scope of new rules: prod, test, all"),
    ("F2", "rename the selected brick"),
    ("Del", "delete the selection"),
    (
        "Ctrl+X, Ctrl+V",
        "cut a brick, paste it on another level (or into the selected brick)",
    ),
    ("Ctrl+Z", "undo your last edit (a new version)"),
    ("Ctrl+Shift+Z / Ctrl+Y", "redo"),
    (
        "Alt+\u{2190} / Alt+\u{2192}",
        "back / forward through levels",
    ),
    ("Backspace", "out to the parent level"),
    ("+ - 0", "zoom in, out, fit"),
    ("Esc", "cancel the drag, tool, cut or form"),
    (
        "drag a brick",
        "onto a brick: into it; hold on a group: open it; onto a lane: retag",
    ),
    (
        "drag a port",
        "onto a sibling or tag lane: new rule of that type",
    ),
    (
        "drag an arrowhead",
        "of the selected rule onto another brick: change its target",
    ),
];

/// The sheet (nothing while closed).
pub fn sheet(ed: Edit) -> View {
    if !ed.help.get() {
        return View::default();
    }
    let rows: Vec<View> = KEYS
        .iter()
        .map(|&(k, what)| view! { tr { td { kbd { (k) } } td { (what) } } })
        .collect();
    let rows = View::from(rows);
    view! {
        div(id="edhelp", class="edpop center", on:mousedown=stop, on:click=stop) {
            div(class="row") {
                b { "Editor shortcuts" }
                button(class="tb", title="Close (Esc or ?)", on:click=move |_: Click| ed.help.set(false)) { "\u{2715}" }
            }
            table { (rows) }
        }
    }
}
