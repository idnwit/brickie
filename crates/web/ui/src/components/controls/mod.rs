//! The header: level breadcrumb, presets, data layers, fit/reload, measure and the finding
//! counts.

mod boundary;
mod crumbs;
mod measure;

pub use boundary::boundary;
pub use crumbs::crumbs;
mod presets;
mod toggles;

use crate::platform;
use crate::state::*;
use presets::{presets, vscope};
use sycamore::prelude::*;
use toggles::layers_menu;

/// Findings among the layers that are on.
fn counts(cx: &Cx) -> impl Fn() -> View + 'static {
    let cx = cx.clone();
    move || {
        let shown = cx.eval.with(|ev| ev.von.iter().filter(|v| **v).count());
        let total = cx.m.diags.len();
        let hidden = if shown < total {
            format!(" \u{00B7} {} hidden by layers", total - shown)
        } else {
            String::new()
        };
        let tip = format!(
            "Findings of the data layers that are on ({shown} of {total}). Brick badges count all layers."
        );
        view! {
            span(title=tip) { b(class="err") { (shown.to_string()) } (if shown == 1 { " error" } else { " errors" }) (hidden) }
        }
    }
}

pub fn header(cx: &Cx) -> View {
    let (vscope, menu) = (vscope(cx), layers_menu(cx));
    let c = cx.clone();
    let fit = move |_: Click| {
        if let Some(s) = c.stage.get() {
            c.view.set(c.fit(s));
        }
    };
    // Bricks stay where they are dragged; this puts them back where the layout placed them (in
    // edit mode also forgetting the level's stored positions).
    let c = cx.clone();
    let relayout = move |_: Click| {
        if c.editing() {
            return crate::components::editor::relayout(&c);
        }
        c.drag.set(Default::default());
        if let Some(s) = c.stage.get() {
            c.view.set(c.fit(s));
        }
    };
    // Design mode reads the design files alone: no finding counts, presets or measure controls.
    // Build them only when shown: an element built but never inserted breaks hydration (its key is
    // taken on the server and the client but missing from the page).
    let code = !cx.editing();
    let counts = if code {
        View::from(counts(cx))
    } else {
        View::default()
    };
    let design = crate::components::editor::design_button(cx);
    let measure = code.then(|| measure::measure(&cx.m.cache, cx.bus.job));
    let pill = code.then(|| measure::pill(cx.bus.pill));
    let presets = code.then(|| presets(cx));
    view! {
        header {
            b { "brickie" }
            (presets)
            (vscope)
            (menu)
            button(id="fit", title="Fit the graph in the stage", on:click=fit) { "Fit" }
            button(id="relayout", title="Lay the level out again: moved bricks go back where the automatic layout places them", on:click=relayout) { "Auto layout" }
            button(id="reload", on:click=move |_: Click| platform::reload()) { "Reload" }
            (design)
            (measure)
            (pill)
            span(class="cnt") { (counts) }
        }
    }
}
