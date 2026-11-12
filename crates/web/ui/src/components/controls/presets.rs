//! The preset buttons (Architecture / Violations / All / Custom) and the Violations scope.

use crate::state::*;
use sycamore::prelude::*;

/// `#preset`: one link per preset; Custom is chosen by changing a data layer.
pub fn presets(cx: &Cx) -> View {
    let items: Vec<View> = PRESETS
        .iter()
        .map(|(p, name)| {
            let preset = *p;
            let disabled = preset == "custom";
            let (c1, c2) = (cx.clone(), cx.clone());
            let cls = move || {
                let on = c1.params.with(|q| q.preset() == preset);
                format!("segb{}", if on { " on" } else { "" })
            };
            let href = with_preset(&cx.params.get_clone(), preset).url();
            view! {
                a(class=cls, href=href, title=if disabled { "Chosen when you change a data layer" } else { "" }, on:click=move |e: Click| {
                    e.prevent_default();
                    if !disabled {
                        c2.go(with_preset(&c2.params.get_clone(), preset));
                    }
                }) { (name.to_string()) }
            }
        })
        .collect();
    let items = View::from(items);
    view! { span(class="seg", id="preset") { (items) } }
}

/// `#vscope`: which finding group the Violations preset shows (only in that preset).
pub fn vscope(cx: &Cx) -> View {
    let cx = cx.clone();
    let body = move || {
        if cx.params.with(|p| p.preset() != "viol") {
            return View::default();
        }
        let opts: Vec<View> = [
            ("all", "all"),
            ("deps", "deps"),
            ("cx", "complexity"),
            ("stale", "stale"),
            ("iface", "interface"),
        ]
        .into_iter()
        .map(|(v, t)| {
            nav_link(
                &cx,
                t.into(),
                "segb sm",
                move |p| p.vscope() == v,
                move |p| p.with(|p| p.vs = v.to_string()),
            )
        })
        .collect();
        let opts = View::from(opts);
        view! { span(class="seg sm", title="Which finding group the Violations preset shows") { (opts) } }
    };
    view! { (body) }
}
