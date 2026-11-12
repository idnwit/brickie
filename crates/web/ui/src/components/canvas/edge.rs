//! One edge: halo, line, hit area, label and mark (`renderGraph` edge part, `edgeOn`).

use super::style::{EdgeFlags, edge_width};
use crate::state::*;
use brickie_web_api::end_name;
use sycamore::prelude::*;

const KIND_ABBR: [(&str, &str); 3] = [
    ("uses", "uses"),
    ("implements", "impl"),
    ("reexports", "reexp"),
];

/// `KDASH`.
pub fn dash(kind: &str) -> &'static str {
    match kind {
        "implements" => "10 5",
        "reexports" => "1.5 4.5",
        _ => "",
    }
}

/// What the evaluation says about edge `ei`, for its classes and markers.
fn edge_flags(cx: &Cx, ei: usize) -> EdgeFlags {
    let edge = &cx.m.g.edges[ei];
    let (arch, item) = cx
        .params
        .with(|p| (p.preset() == "arch", !p.item.is_empty()));
    cx.eval.with(|ev| {
        let focus = ev.focus.as_ref();
        // Hidden unless some layer shows it and the focus keeps it.
        let st = ev.edge[ei].filter(|_| focus.is_none_or(|f| f.keep.contains(&ei)));
        EdgeFlags {
            st,
            wall: edge.wall,
            err: st == Some(EStat::Debt) && is_err(cx, ei),
            arch,
            hot: focus.is_some_and(|f| f.hot.contains(&ei)),
            lab: focus.is_some_and(|f| !f.group),
            sel: ev.sel == Sel::Edge(ei) && !item,
            hov: cx
                .hover
                .get()
                .is_some_and(|h| finding_focus(&cx.m.g, &cx.m.diags[h]).hot.contains(&ei)),
        }
    })
}

/// Whether an open error (or no finding at all, as for cycle members) backs the edge.
fn is_err(cx: &Cx, ei: usize) -> bool {
    let ids = &cx.m.g.edges[ei].diags;
    let mut cards =
        cx.m.cards
            .iter()
            .filter(|c| ids.contains(&c.d.id))
            .peekable();
    cards.peek().is_none() || cards.any(|c| !c.accepted.get() && c.d.level == "error")
}

/// One edge: halo, line, hit area, label and mark. Visible only while some layer shows it and
/// the focus keeps it (`edgeOn`, `focusSpec`).
pub fn edge_view(cx: &Cx, ei: usize, routes: ReadSignal<Vec<Option<(String, f64, f64)>>>) -> View {
    let kind = cx.m.g.edges[ei].kind.clone();
    let abbr = KIND_ABBR
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("", |(_, a)| *a);
    let tip = {
        let g = &cx.m.g;
        let e = &g.edges[ei];
        let wall = if e.wall {
            "; private: stops at the wall of this level"
        } else {
            ""
        };
        format!(
            "{} -> {} ({}, {} reference(s){wall})",
            end_name(g, ei, e.from),
            end_name(g, ei, e.to),
            e.kind,
            e.count
        )
    };
    let width = edge_width(cx.m.g.edges[ei].count);
    // One evaluation per edge; the four attributes below read the memo.
    let flags = {
        let c = cx.clone();
        create_memo(move || edge_flags(&c, ei))
    };
    let cls = {
        let kind = kind.clone();
        move || flags.get().class(&kind)
    };
    let marker = {
        let kind = kind.clone();
        move || flags.get().marker_end(&kind)
    };
    let start = {
        let kind = kind.clone();
        move || flags.get().marker_start(&kind)
    };
    let mark_text = move || flags.get().mark();
    let (d1, d2, d3) = (routes, routes, routes);
    let d_vis = move || d1.with(|r| r[ei].as_ref().map(|x| x.0.clone()).unwrap_or_default());
    let d_halo = move || d2.with(|r| r[ei].as_ref().map(|x| x.0.clone()).unwrap_or_default());
    let d_hit = move || d3.with(|r| r[ei].as_ref().map(|x| x.0.clone()).unwrap_or_default());
    let mid = move || routes.with(|r| r[ei].as_ref().map_or((0.0, 0.0), |x| (x.1, x.2)));
    let (lx, ly) = (move || mid().0.to_string(), move || (mid().1).to_string());
    let (mx, my) = (
        move || mid().0.to_string(),
        move || (mid().1 + 5.0).to_string(),
    );
    let dasharray = dash(&kind);
    let cap = if kind == "reexports" { "round" } else { "butt" };
    let c = cx.clone();
    let click = move |e: Click| {
        e.stop_propagation();
        c.select(Sel::Edge(ei));
    };
    view! {
        g(class=cls, "data-id"=ei.to_string(), style=width) {
            svg_title { (tip) }
            path(d=d_halo, class="halo")
            path(d=d_vis, class="vis", "stroke-dasharray"=dasharray, "stroke-linecap"=cap,
                "marker-end"=marker, "marker-start"=start)
            path(d=d_hit, class="hit", on:click=click)
            text(x=lx, y=ly, "text-anchor"="middle", class="lab") { (abbr) }
            text(x=mx, y=my, "text-anchor"="middle", class="mark") { (mark_text) }
        }
    }
}
