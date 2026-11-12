//! The header: level breadcrumb, presets, data layers, fit/reload and the finding counts.

mod crumbs;

pub use crumbs::crumbs;
mod presets;
mod toggles;

use crate::platform;
use crate::state::*;
use presets::{presets, vscope};
use sycamore::prelude::*;
use toggles::layers_menu;

fn plural(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// Open errors, warnings and accepted findings among the layers that are on.
fn counts(cx: &Cx) -> impl Fn() -> View + 'static {
    let cx = cx.clone();
    move || {
        let (mut e, mut w, mut acc, mut shown) = (0, 0, 0, 0);
        for (i, c) in cx.m.cards.iter().enumerate() {
            if !cx.eval.with(|ev| ev.von[i]) {
                continue;
            }
            shown += 1;
            if c.accepted.get() {
                acc += 1;
            } else if c.d.level == "error" {
                e += 1;
            } else {
                w += 1;
            }
        }
        let total = cx.m.cards.len();
        let hidden = if shown < total {
            format!(" \u{00B7} {} hidden by layers", total - shown)
        } else {
            String::new()
        };
        let tip = format!(
            "Findings of the data layers that are on ({shown} of {total}); accepted ones are not open. Brick badges count all layers."
        );
        view! {
            span(title=tip) { b(class="err") { (e.to_string()) } " open errors \u{00B7} " (plural(w, "warning")) " \u{00B7} " (acc.to_string()) " accepted" (hidden) }
        }
    }
}

pub fn header(cx: &Cx) -> View {
    let (presets, vscope, menu) = (presets(cx), vscope(cx), layers_menu(cx));
    let c = cx.clone();
    let fit = move |_: Click| {
        if let Some(s) = c.stage.get() {
            c.view.set(fit_view(&c.m.layout, s));
        }
    };
    let counts = counts(cx);
    view! {
        header {
            b { "brickie" }
            (presets)
            (vscope)
            (menu)
            button(id="fit", title="Fit the graph in the stage", on:click=fit) { "Fit" }
            button(id="reload", on:click=move |_: Click| platform::reload()) { "Reload" }
            span(class="cnt") { (counts) }
        }
    }
}
