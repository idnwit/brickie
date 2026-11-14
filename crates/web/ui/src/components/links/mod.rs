//! Unused links on the canvas, in view mode and in the editor: one line per (from, to) brick
//! pair, dashed like the edge kind, in the warning colour with a `!`. A yellow `?` marks a rule without a reason, `A` an agent's rule.
//! The reason shows as a label on hover or selection only. In the editor a click selects the
//! rule; in view mode it picks the link (a tap on touch).

mod pair;
mod spot;

pub use pair::ends;

use crate::layout::{Pt, Rect, Route};
use crate::state::*;
use brickie_web_api::UnusedLink;
use brickie_web_api::design::Op as DOp;
use pair::{Pair, pairs};
use spot::mark_spot;
use sycamore::prelude::*;

/// Horizontal step between the marks of one link (world units).
const SLOT: f64 = 18.0;

/// Whether an agent's version since your last edit added rule `e`.
fn by_agent(cx: &Cx, e: &UnusedLink) -> bool {
    let Some(d) = cx.m.design.as_deref() else {
        return false;
    };
    agent_ops(&d.history).into_iter().any(|o| {
        matches!(o, DOp::AddRule { file, rule } if *file == e.file && rule.from == e.from && rule.to == e.to)
    })
}

/// The route of pair `a -> b`: the layout's (around the boxes, from side ports), or a plain
/// cubic from the right to the left side when the pair is not routed.
fn path_of(cx: &Cx, a: usize, b: usize, p: &[Rect]) -> Route {
    if let Some(r) = cx.unused_route(a, b, p) {
        return r;
    }
    let (x1, y1) = (p[a].x + p[a].w, p[a].y + p[a].h / 2.0);
    let (x2, y2) = (p[b].x, p[b].y + p[b].h / 2.0);
    let mx = (x1 + x2) / 2.0;
    let at = |x, y| Pt { x, y };
    Route {
        segs: vec![[at(x1, y1), at(mx, y1), at(mx, y2), at(x2, y2)]],
    }
}

/// The dash of the pair's edge kind (`implements`, `reexports`); `None` keeps the default dash.
fn kind_dash(e: &UnusedLink) -> Option<&'static str> {
    match e.kinds.as_slice() {
        [k] if k == "implements" || k == "reexports" => Some(crate::components::dash(k)),
        _ => None,
    }
}

fn glyph(x: f64, y: f64, g: &str) -> View {
    let (px, py, ty) = (x.to_string(), y.to_string(), (y + 4.0).to_string());
    let (g, px2) = (g.to_string(), px.clone());
    view! { g(class="qmark") { circle(cx=px2, cy=py, r="8") text(x=px, y=ty, "text-anchor"="middle") { (g) } } }
}

/// One pair: line, hit area, marks and the hover label.
fn draw(
    cx: &Cx,
    pr: &Pair,
    p: &[Rect],
    marks: &mut Vec<Pt>,
    picked: Signal<Option<(usize, usize)>>,
) -> View {
    let lead = pr.lead();
    let ty = LinkType::of_rule(&lead.kinds, &lead.action);
    let agent = pr.rules.iter().any(|e| by_agent(cx, e));
    let route = path_of(cx, pr.a, pr.b, p);
    let d = route.d();
    let at = mark_spot(&route, p, marks);
    let mut glyphs: Vec<&str> = Vec::new();
    glyphs.push("!");
    if pr.no_reason() {
        glyphs.push("?");
    }
    let mut x = at.x;
    let mut shown: Vec<View> = Vec::new();
    for g in &glyphs {
        shown.push(glyph(x, at.y, g));
        x += SLOT;
    }
    if agent {
        let t = view! { text(class="abadge", x=x.to_string(), y=(at.y + 4.0).to_string(), "text-anchor"="middle") { "A" } };
        shown.push(t);
        x += SLOT;
    }
    let label = pr.label().map(|t| {
        let lx = if x > at.x {
            x - SLOT + 14.0
        } else {
            at.x + 10.0
        };
        view! { text(class="rlabel", x=lx.to_string(), y=(at.y + 4.0).to_string()) { (t) } }
    });
    let refs: Vec<RuleRef> = pr
        .rules
        .iter()
        .map(|e| RuleRef {
            file: e.file.clone(),
            key: unused_key(e),
        })
        .collect();
    let (ed, editing, key) = (cx.ed, cx.editing(), (pr.a, pr.b));
    let (sel_refs, dash) = (refs.clone(), kind_dash(lead));
    let cls = move || -> String {
        let sel = editing
            && ed
                .rule
                .with(|x| x.as_ref().is_some_and(|r| sel_refs.contains(r)));
        let pick = picked.get() == Some(key);
        let agent = if agent { " agent" } else { "" };
        let (sel, pick) = (
            if sel { " sel" } else { "" },
            if pick { " picked" } else { "" },
        );
        format!("pedge plink l-{}{agent}{sel}{pick}", ty.name())
    };
    let click = move |ev: Click| {
        ev.stop_propagation();
        picked.set((picked.get_untracked() != Some(key)).then_some(key));
        if editing {
            // A pair of several rules: each click selects the next one.
            let at = ed
                .rule
                .with_untracked(|x| x.as_ref().and_then(|r| refs.iter().position(|q| q == r)));
            ed.rule
                .set(refs.get(at.map_or(0, |i| (i + 1) % refs.len())).cloned());
            ed.tab.set("inspect");
        }
    };
    let dash_style = dash.map_or_else(String::new, |s| format!("stroke-dasharray:{s}"));
    let d2 = d.clone();
    let shown = View::from(shown);
    let tip = pr.tip();
    view! {
        g(class=cls, on:click=click) {
            svg_title { (tip) }
            path(class="hit", d=d2)
            path(class="line", d=d, style=dash_style)
            (shown)
            (label)
        }
    }
}

/// The links layer of the canvas (inside the pan/zoom group).
pub fn layer(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    let c = cx.clone();
    let picked: Signal<Option<(usize, usize)>> = create_signal(None);
    let body = move || {
        let p = pos.get_clone();
        let mut marks = Vec::new();
        let items: Vec<View> = pairs(&c)
            .iter()
            .filter(|pr| pr.a < p.len() && pr.b < p.len())
            .map(|pr| draw(&c, pr, &p, &mut marks, picked))
            .collect();
        let items = View::from(items);
        view! { g(class="plinks") { (items) } }
    };
    view! { (body) }
}
