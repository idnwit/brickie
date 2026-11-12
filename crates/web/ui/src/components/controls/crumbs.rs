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
    let hist = history(cx);
    view! {
        (hist)
        nav(class="crumbs overlay", "aria-label"="Level", title=tip,
            on:mousedown=stop, on:click=stop, on:dblclick=stop) { (out) }
    }
}

/// Back / Forward through the view history (`Alt+Left` / `Alt+Right`, mouse buttons 4 / 5):
/// the browser's history, so both stay in step.
fn history(cx: &Cx) -> View {
    let nav = cx.nav;
    let stop = |e: web_sys::MouseEvent| e.stop_propagation();
    let button = move |d: i32, glyph: &'static str, word: &'static str| {
        let to = move || nav.with(|n| if d < 0 { n.back() } else { n.forward() }.map(nav_label));
        let tip = move || match to() {
            Some(l) => format!(
                "{word} to {l} (Alt+{})",
                if d < 0 { "Left" } else { "Right" }
            ),
            None => format!("Nothing to go {}", word.to_lowercase()),
        };
        let off = move || to().is_none();
        view! {
            button(class="navbtn", r#type="button", title=tip, "aria-label"=word, disabled=off,
                on:click=move |_: web_sys::MouseEvent| crate::platform::history_go(d)) { (glyph) }
        }
    };
    let (b, f) = (
        button(-1, "\u{2190}", "Back"),
        button(1, "\u{2192}", "Forward"),
    );
    view! {
        span(class="navh overlay", on:mousedown=stop, on:click=stop, on:dblclick=stop) { (b) (f) }
    }
}
