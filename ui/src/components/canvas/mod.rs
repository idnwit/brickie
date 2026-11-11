//! The canvas (`renderGraph` and the stage handlers of the original): rows, edges and bricks from
//! the one layout computed up front. Toggling layers, presets or selection only changes classes
//! and styles of existing elements; nothing is re-laid out.

mod brick;
mod defs;
mod edge;
mod pointer;
mod rows;

use brick::node_view;
use defs::defs;
pub use edge::dash;
use edge::edge_view;
use pointer::pointer;
use rows::row_labels;

use crate::layout::Rect;
use crate::platform;
use crate::state::*;
use sycamore::prelude::*;

/// The stage: pan (drag), zoom (wheel), drag-box select (shift-drag), brick drag, click on the
/// background clears the selection.
pub fn canvas(cx: &Cx) -> View {
    pointer(cx);
    let l = &cx.m.layout;
    let n_edges = cx.m.g.edges.len();
    let n_bricks = cx.m.g.bricks.len();
    let pos: ReadSignal<Vec<Rect>> = {
        let cx = cx.clone();
        create_memo(move || cx.drag.with(|d| positions(&cx.m.layout, d)))
    };
    let routes: ReadSignal<Vec<Option<(String, f64, f64)>>> = {
        let cx = cx.clone();
        create_memo(move || {
            let p = pos.get_clone();
            (0..cx.m.g.edges.len())
                .map(|e| {
                    cx.m.layout.path(e, &p).map(|r| {
                        let m = r.mid();
                        (r.d(), m.x, m.y)
                    })
                })
                .collect()
        })
    };
    // Edges that can be violating draw in a second group above the allowed ones; the split is
    // static (from the data), so presets and layers never move an element between groups.
    let (top, low): (Vec<usize>, Vec<usize>) =
        (0..n_edges).partition(|&e| !cx.m.g.edges[e].diags.is_empty());
    let low: Vec<View> = low.into_iter().map(|e| edge_view(cx, e, routes)).collect();
    let top: Vec<View> = top.into_iter().map(|e| edge_view(cx, e, routes)).collect();
    let (low, top) = (View::from(low), View::from(top));
    let nodes: Vec<View> = (0..n_bricks).map(|b| node_view(cx, b, pos)).collect();
    let nodes = View::from(nodes);
    let rows = row_labels(cx);
    let defs = defs();
    let (w, h) = (l.w, l.h);
    let stage = cx.stage;
    let vb = move || match stage.get() {
        Some((sw, sh)) => format!("0 0 {sw} {sh}"),
        None => format!("0 0 {w} {h}"),
    };
    let view = cx.view;
    let vp = move || {
        let v = view.get();
        format!("translate({},{}) scale({})", v.x, v.y, v.k)
    };
    let c = cx.clone();
    let down = move |e: Click| {
        if e.button() != 0 {
            return;
        }
        let Some((l, t, _, _)) = platform::stage_rect() else {
            return;
        };
        c.moved.set(false);
        if e.shift_key() {
            let (x, y) = (e.client_x() as f64 - l, e.client_y() as f64 - t);
            c.op.set(Some(Op::Box {
                x0: x,
                y0: y,
                x1: x,
                y1: y,
            }));
        } else {
            let v = c.view.get();
            c.op.set(Some(Op::Pan {
                sx: e.client_x() as f64,
                sy: e.client_y() as f64,
                vx: v.x,
                vy: v.y,
            }));
        }
    };
    let c = cx.clone();
    let wheel = move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let Some((l, t, _, _)) = platform::stage_rect() else {
            return;
        };
        let (mx, my) = (e.client_x() as f64 - l, e.client_y() as f64 - t);
        let v = c.view.get();
        let k = (v.k * (-e.delta_y() * 0.0015).exp()).clamp(0.15, 3.0);
        c.view.set(Vw {
            x: mx - (mx - v.x) * (k / v.k),
            y: my - (my - v.y) * (k / v.k),
            k,
        });
    };
    let c = cx.clone();
    let bg_click = move |_: Click| {
        if c.moved.get() {
            c.moved.set(false);
            return;
        }
        let p = c.params.get_clone();
        let item_or_sel =
            !p.item.is_empty() || !p.brick.is_empty() || !p.group.is_empty() || !p.edge.is_empty();
        if item_or_sel {
            c.select(Sel::None);
        }
    };
    let c = cx.clone();
    let pan_cls = move || {
        if matches!(c.op.get(), Some(Op::Pan { .. })) {
            "pan"
        } else {
            ""
        }
    };
    let c = cx.clone();
    let sel_box = move || match c.op.get() {
        Some(Op::Box { x0, y0, x1, y1 }) => format!(
            "left:{}px;top:{}px;width:{}px;height:{}px",
            x0.min(x1),
            y0.min(y1),
            (x1 - x0).abs(),
            (y1 - y0).abs()
        ),
        _ => "display:none".to_string(),
    };
    view! {
        div(id="stage", class=pan_cls, on:mousedown=down, on:wheel=wheel, on:click=bg_click) {
            svg(id="svg", "viewBox"=vb) {
                (defs)
                g(id="vp", transform=vp) {
                    (rows)
                    g(class="lay") { (low) }
                    g(class="lay top") { (top) }
                    g { (nodes) }
                }
            }
            div(class="selbox", style=sel_box)
        }
    }
}
