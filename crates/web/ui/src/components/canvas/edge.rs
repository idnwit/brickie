//! One edge: halo, line, hit area, label and mark (`renderGraph` edge part, `edgeOn`).

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

/// One edge: halo, line, hit area, label and mark. Visible only while some layer shows it and
/// the focus keeps it (`edgeOn`, `focusSpec`).
pub fn edge_view(cx: &Cx, ei: usize, routes: ReadSignal<Vec<Option<(String, f64, f64)>>>) -> View {
    let kind = cx.m.g.edges[ei].kind.clone();
    let abbr = KIND_ABBR
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("", |(_, a)| *a);
    let (tip, wall, width) = {
        let g = &cx.m.g;
        let e = &g.edges[ei];
        let wall = if e.wall {
            "; private: stops at the wall of this level"
        } else {
            ""
        };
        let tip = format!(
            "{} -> {} ({}, {} reference(s){wall})",
            end_name(g, ei, e.from),
            end_name(g, ei, e.to),
            e.kind,
            e.count
        );
        // Width grows with the number of references (log scale, at most 2.6x).
        let k = (1.0 + 0.35 * (e.count.max(1) as f64).log2()).min(2.6);
        (tip, e.wall, format!("--k:{k:.2}"))
    };
    // The status the edge is drawn with, `None` when hidden.
    let shown = {
        let cx = cx.clone();
        move || {
            cx.eval.with(|ev| {
                let st = ev.edge[ei]?;
                match &ev.focus {
                    Some(f) if !f.keep.contains(&ei) => None,
                    _ => Some(st),
                }
            })
        }
    };
    // Kind colours show only in the Architecture preset; elsewhere allowed edges are grey.
    let arch = {
        let cx = cx.clone();
        move || cx.params.with(|p| p.preset() == "arch")
    };
    // Whether an open error (or no finding at all, as for cycle members) backs the edge.
    let is_err = {
        let cx = cx.clone();
        move || {
            let ids = &cx.m.g.edges[ei].diags;
            let mut cards =
                cx.m.cards
                    .iter()
                    .filter(|c| ids.contains(&c.d.id))
                    .peekable();
            cards.peek().is_none() || cards.any(|c| !c.accepted.get() && c.d.level == "error")
        }
    };
    let cls = {
        let (cx, shown, arch, is_err) = (cx.clone(), shown, arch, is_err.clone());
        let kind = kind.clone();
        move || {
            let Some(st) = shown() else {
                return "edge off".to_string();
            };
            let mut c = format!("edge k-{kind}");
            if wall {
                c += " wall";
            }
            c += match st {
                EStat::Ok => " ok",
                EStat::Accepted => " acc",
                EStat::Debt if is_err() => " err",
                EStat::Debt => " warn",
            };
            if arch() {
                c += " kc";
            }
            cx.eval.with(|ev| {
                if let Some(f) = &ev.focus {
                    if f.hot.contains(&ei) {
                        c += " hot";
                    }
                    if !f.group {
                        c += " lab";
                    }
                }
                if ev.sel == Sel::Edge(ei) && cx.params.with(|p| p.item.is_empty()) {
                    c += " sel";
                }
            });
            if let Some(h) = cx.hover.get()
                && finding_focus(&cx.m.g, &cx.m.diags[h]).hot.contains(&ei)
            {
                c += " hov";
            }
            c
        }
    };
    // Arrowheads: violating and accepted edges get their own colour; allowed ones the kind
    // colour in Architecture and grey elsewhere.
    let marker = {
        let kind = kind.clone();
        move || {
            if wall {
                return String::new();
            }
            let id = match shown() {
                Some(EStat::Accepted) => "acc".to_string(),
                Some(EStat::Debt) if is_err() => "viol".to_string(),
                Some(EStat::Debt) => "warn".to_string(),
                _ if arch() => kind.clone(),
                _ => format!("n-{kind}"),
            };
            format!("url(#m-{id})")
        }
    };
    let start = {
        let kind = kind.clone();
        move || {
            if kind != "reexports" {
                String::new()
            } else if arch() {
                "url(#t-reexports)".to_string()
            } else {
                "url(#t-n-reexports)".to_string()
            }
        }
    };
    let mark_text = move || match shown() {
        Some(EStat::Debt) => "\u{2717}",
        Some(EStat::Accepted) => "\u{2713}",
        _ => "",
    };
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
