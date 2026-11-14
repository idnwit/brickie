//! The canvas (`renderGraph` and the stage handlers of the original): rows, edges and bricks from
//! the one layout computed up front. Toggling layers, presets or selection only changes classes
//! and styles of existing elements; nothing is re-laid out.

mod boundary;
mod brick;
mod defs;
mod edge;
mod frame;
mod pointer;
mod route;
mod rows;
mod stage;
mod style;
mod zoom_icon;

use brick::node_view;
use defs::defs;
pub use edge::dash;
use edge::edge_view;
pub use frame::frame_rect;
use frame::frame_view;
pub use pointer::pointer;
use route::{Routes, routes};
use rows::row_labels;

use crate::components::editor;
use crate::layout::Rect;
use crate::platform;
use crate::state::*;
use sycamore::prelude::*;

/// The measured stage: record its size and fit the view (unless an editor patch kept it).
fn first_fit(c: &Cx, s: (f64, f64)) {
    c.stage.set(Some(s));
    if c.fit {
        c.view.set(c.fit(s));
    }
}

/// The edge groups `(allowed, can-be-violating)`: the latter draws above the former, and the
/// split is static (from the data), so presets and layers never move an element between groups.
fn edge_layers(cx: &Cx, routes: Routes) -> (View, View) {
    let (top, low): (Vec<usize>, Vec<usize>) =
        (0..cx.m.g.edges.len()).partition(|&e| !cx.m.g.edges[e].diags.is_empty());
    let group = |es: Vec<usize>| {
        View::from(
            es.into_iter()
                .map(|e| edge_view(cx, e, routes))
                .collect::<Vec<View>>(),
        )
    };
    (group(low), group(top))
}

/// The stage: pan (drag), zoom (wheel), drag-box select (shift-drag), brick drag, click on the
/// background clears the selection.
pub fn canvas(cx: &Cx) -> View {
    let n_bricks = cx.m.g.bricks.len();
    let pos: ReadSignal<Vec<Rect>> = {
        let cx = cx.clone();
        create_memo(move || cx.drag.with(|d| positions(&cx.m.layout, d)))
    };
    let routes = routes(cx, pos);
    // Edges that can be violating draw in a second group above the allowed ones; the split is
    // static (from the data), so presets and layers never move an element between groups.
    let (low, top) = edge_layers(cx, routes);
    let nodes: Vec<View> = (0..n_bricks).map(|b| node_view(cx, b, pos)).collect();
    let nodes = View::from(nodes);
    let rows = row_labels(cx);
    let frame = frame_view(cx, pos);
    let defs = defs();
    let view = cx.view;
    let vp = move || {
        let v = view.get();
        format!("translate({},{}) scale({})", v.x, v.y, v.k)
    };
    let (down, wheel, bg_click) = (stage::down(cx), stage::wheel(cx), stage::bg_click(cx));
    // The entry transition (`zin`/`zout`) plays once when the level's stage mounts.
    let anim = cx.anim;
    let c = cx.clone();
    let pan_cls = move || stage_class(&c, anim);
    let c = cx.clone();
    let sel_box = move || stage::box_style(c.op.get());
    let crumbs = crate::components::controls::crumbs(cx);
    let bnd = crate::components::controls::boundary(cx);
    let edit = editor::overlay(cx);
    let layer = editor::layer(cx, pos);
    // The svg has no viewBox: one user unit is one CSS pixel of the stage, so pointer
    // positions map straight into the pan/zoom transform. On the next frame (the stage is in
    // the DOM) measure it and fit. Always set the view: after hydration the DOM still shows
    // the server's transform even when the signal already holds the fitted one.
    let c = cx.clone();
    platform::next_frame(move || {
        if let Some((_, _, sw, sh)) = platform::stage_rect() {
            first_fit(&c, (sw, sh));
        }
    });
    view! {
        div(id="stage", class=pan_cls, on:mousedown=down, on:wheel=wheel, on:click=bg_click) {
            svg(id="svg") {
                (defs)
                g(id="vp", transform=vp) {
                    (frame)
                    (rows)
                    g(class="lay") { (low) }
                    g(class="lay top") { (top) }
                    g { (nodes) }
                    (layer)
                }
            }
            div(class="selbox", style=sel_box)
            div(class="ovl") { (crumbs) (bnd) }
            (edit)
        }
    }
}

/// Classes of `#stage`: its entry transition, `pan` while panning, `nodrop` over a refused drop.
fn stage_class(c: &Cx, anim: &str) -> String {
    let no = c.ed.target.with(|t| matches!(t, Some(MoveTarget::No(_))));
    let pan = matches!(c.op.get(), Some(Op::Pan { .. }));
    format!(
        "{}{anim}{}",
        if pan { "pan " } else { "" },
        if no { " nodrop" } else { "" }
    )
}
