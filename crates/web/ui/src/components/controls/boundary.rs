//! The "Outside -> frame | children" toggle next to the breadcrumb (not at the root): whether
//! edges from outside neighbours end at the frame or at the children they reach. Only the
//! params change, so pan, zoom and dragged bricks stay.

use crate::state::*;
use sycamore::prelude::*;

pub fn boundary(cx: &Cx) -> View {
    if cx.level().path == "root" {
        return View::default();
    }
    let opts: Vec<View> = [("frame", "frame"), ("children", "children")]
        .into_iter()
        .map(|(v, t)| {
            nav_link(
                cx,
                t.into(),
                "segb sm",
                move |p| (p.bnd == "frame") == (v == "frame"),
                move |p| {
                    p.with(|p| {
                        p.bnd = if v == "frame" {
                            v.into()
                        } else {
                            String::new()
                        }
                    })
                },
            )
        })
        .collect();
    let opts = View::from(opts);
    let tip = "Edges from outside neighbours: aggregated at the frame of this level, or drawn to the children they reach";
    let stop = |e: web_sys::MouseEvent| e.stop_propagation();
    view! {
        div(class="bnd overlay", title=tip, on:mousedown=stop, on:click=stop, on:dblclick=stop) {
            span(class="muted") { "Outside \u{2192}" }
            span(class="seg sm") { (opts) }
        }
    }
}
