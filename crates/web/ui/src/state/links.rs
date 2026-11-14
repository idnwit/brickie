//! Links that change the params or open the code viewer.

use super::Cx;
use brickie_web_api::Params;
use sycamore::prelude::*;

pub type Click = web_sys::MouseEvent;

/// Plain left click (no new-tab or download modifiers).
pub fn plain(e: &Click) -> bool {
    e.button() == 0 && !(e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key())
}

/// A link that changes the params: a real `href` for no-JS, a signal update when hydrated.
pub fn nav_link(
    cx: &Cx,
    label: String,
    class: &'static str,
    active: impl Fn(&Params) -> bool + 'static,
    change: impl Fn(&Params) -> Params + Clone + 'static,
) -> View {
    let on_click = change.clone();
    let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
    let href = move || change(&c1.params.get_clone()).url();
    let cls = move || {
        if active(&c2.params.get_clone()) {
            format!("{class} on")
        } else {
            class.to_string()
        }
    };
    view! {
        a(href=href, class=cls, on:click=move |e: Click| {
            if plain(&e) {
                e.prevent_default();
                let p = on_click(&c3.params.get_clone());
                c3.go(p);
            }
        }) { (label) }
    }
}

/// A link to a source location: opens the code viewer popup when hydrated, the code viewer in a
/// tab otherwise.
pub fn src_link(
    modal: Signal<Option<String>>,
    href: String,
    text: String,
    class: &'static str,
) -> View {
    let target = href.clone();
    view! {
        a(href=href, class=class, target="_blank", on:click=move |e: Click| {
            if plain(&e) {
                e.prevent_default();
                modal.set(Some(target.clone()));
            }
        }) { (text) }
    }
}
