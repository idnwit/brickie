//! The zoom-in affordance drawn on composite bricks.

use crate::state::*;
use brickie_web_api::BrickView;
use sycamore::prelude::*;

/// A small magnifier with a plus at the start of a composite's tag line (the tag text moves
/// right by [`ZOOM_ICON_W`]): it has a level inside. A click on it zooms in directly (a
/// double-click on the brick does too).
pub fn zoom_icon(cx: &Cx, b: &BrickView) -> View {
    if !b.composite() || b.ghost {
        return View::default();
    }
    let tr = "translate(8 37)";
    let path = b.path.clone();
    let c = cx.clone();
    let click = move |e: Click| {
        e.stop_propagation();
        c.go(c.params.with(|p| p.at_level(&path)));
    };
    let stop = |e: Click| e.stop_propagation();
    view! {
        g(class="zicon", transform=tr, on:mousedown=stop, on:click=click) {
            svg_title { "zoom in (or double-click the brick)" }
            rect(class="zhit", x="-3", y="-3", width="18", height="18")
            circle(cx="5", cy="5", r="4.5")
            line(x1="8.3", y1="8.3", x2="12", y2="12")
            line(x1="3", y1="5", x2="7", y2="5")
            line(x1="5", y1="3", x2="5", y2="7")
        }
    }
}

/// Room the icon takes at the start of the tag line (px).
pub const ZOOM_ICON_W: f64 = 17.0;
