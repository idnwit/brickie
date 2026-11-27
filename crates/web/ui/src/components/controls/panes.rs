//! The header's side-panel toggles: hide the findings list (`[`) or the detail panel (`]`) to
//! give the canvas their width.

use crate::platform;
use crate::state::*;
use sycamore::prelude::*;

/// Show or hide one side panel (`left`: the findings list, else the detail panel), then fit
/// the graph in the resized stage once the new layout is in place.
pub fn toggle_pane(c: &Cx, left: bool) {
    c.panes.update(|p| {
        if left {
            p.left = !p.left;
        } else {
            p.right = !p.right;
        }
    });
    let c = c.clone();
    platform::next_frame(move || {
        if let Some(r) = platform::stage_rect() {
            c.stage.set(Some((r.2, r.3)));
            c.view.set(c.fit((r.2, r.3)));
        }
    });
}

/// One toggle button: highlighted while its panel is shown.
fn pane_button(cx: &Cx, left: bool) -> View {
    let panes = cx.panes;
    let cls = move || {
        let on = panes.with(|p| if left { p.left } else { p.right });
        if on { "pane on" } else { "pane" }
    };
    let (label, tip) = if left {
        ("\u{25E7}", "Show or hide the findings list ([)")
    } else {
        ("\u{25E8}", "Show or hide the detail panel (])")
    };
    let c = cx.clone();
    let click = move |_: Click| toggle_pane(&c, left);
    view! { button(class=cls, title=tip, on:click=click) { (label) } }
}

/// The left and right panel toggles.
pub fn pane_buttons(cx: &Cx) -> View {
    let (l, r) = (pane_button(cx, true), pane_button(cx, false));
    view! { span(class="panes") { (l) (r) } }
}
