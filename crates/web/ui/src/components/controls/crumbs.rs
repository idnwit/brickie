//! The level breadcrumb (`root › web › api`), an overlay at the top-left of the stage: every ancestor is a link that zooms out to it.

use crate::state::*;
use sycamore::prelude::*;

pub fn crumbs(cx: &Cx) -> View {
    let path = cx.level().path.clone();
    let parts: Vec<&str> = path.split('/').collect();
    let mut out: Vec<View> = Vec::new();
    for i in 0..parts.len() {
        let name = parts[i].to_string();
        if i > 0 {
            out.push(view! { span(class="csep") { "\u{203A}" } });
        }
        if i + 1 == parts.len() {
            out.push(view! { span(class="ccur", "aria-current"="page") { (name) } });
        } else {
            let target = parts[..=i].join("/");
            out.push(nav_link(
                cx,
                name,
                "crumb",
                |_| false,
                move |p| p.at_level(&target),
            ));
        }
    }
    let out = View::from(out);
    let tip = "Level of the brick tree. Double-click a framed brick to zoom in; Backspace or Esc zooms out.";
    // An overlay on the stage: keep its clicks from panning or clearing the selection.
    let stop = |e: web_sys::MouseEvent| e.stop_propagation();
    view! {
        nav(class="crumbs overlay", "aria-label"="Level", title=tip,
            on:mousedown=stop, on:click=stop, on:dblclick=stop) { (out) }
    }
}
