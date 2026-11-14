//! The editor's layer of the canvas (inside the pan/zoom group): agent badges (what an agent changed
//! since your last edit), typed ports on the hot
//! brick, the rubber band of a port drag with its valid and refused targets, and the target
//! of a brick being moved.

use super::connect;
use super::marks::{drag_label, move_mark, new_lane_hint};
use crate::layout::{Pt, Rect};
use crate::state::*;
use sycamore::prelude::*;

/// The agent badge anchors: the top-left corner of each brick an agent changed since your
/// last edit.
fn badge_spots(cx: &Cx, p: &[Rect]) -> Vec<Pt> {
    let Some(ch) = cx.m.design.as_deref() else {
        return Vec::new();
    };
    let file = ch.files.get(&cx.level().path);
    let ops = agent_ops(&ch.history);
    (0..cx.m.g.bricks.len().min(p.len()))
        .filter(|&i| {
            let b = &cx.m.g.bricks[i];
            (ops.iter()).any(|o| {
                Some(&o.file().to_string()) == file && op_brick(o) == Some(b.name.as_str())
            })
        })
        .map(|i| Pt {
            x: p[i].x - 6.0,
            y: p[i].y - 6.0,
        })
        .collect()
}

fn badges(spots: &[Pt]) -> View {
    let out: Vec<View> = (spots.iter())
        .map(|&Pt { x, y }| view! { g(class="ebadge agent") { circle(cx=x.to_string(), cy=y.to_string(), r="9") text(x=x.to_string(), y=(y + 4.0).to_string(), "text-anchor"="middle") { "A" } } })
        .collect();
    View::from(out)
}

/// The typed ports on the right edge of brick `b`.
fn ports(cx: &Cx, b: usize, r: Rect) -> View {
    let ed = cx.ed;
    let scope = ed.scope.get();
    let items: Vec<View> = LINK_TYPES
        .iter()
        .enumerate()
        .map(|(i, &ty)| {
            let (x, y) = (r.x + r.w + 11.0, r.y + 7.0 + i as f64 * (r.h - 14.0) / 3.0);
            let from = cx.m.g.bricks[b].name.clone();
            let down = move |e: Click| {
                e.stop_propagation();
                e.prevent_default();
                connect::start(ed, from.clone(), ty, x, y);
            };
            let tip = format!("drag: {} link ({scope}) to a sibling or a tag lane", ty.name());
            view! {
                g(class=format!("lport l-{}", ty.name()), "data-port"=ty.name(), on:mousedown=down) {
                    svg_title { (tip) }
                    circle(cx=x.to_string(), cy=y.to_string(), r="7")
                    text(x=x.to_string(), y=(y + 3.5).to_string(), "text-anchor"="middle") { (ty.glyph()) }
                }
            }
        })
        .collect();
    View::from(items)
}

fn ported(cx: &Cx) -> Vec<usize> {
    let ed = cx.ed;
    if ed.link_type.get().is_some() {
        return (0..cx.m.g.bricks.len())
            .filter(|&i| !cx.m.g.bricks[i].ghost)
            .collect();
    }
    let sel = match cx.eval.with(|ev| ev.sel.clone()) {
        Sel::Brick(b) => Some(b),
        _ => None,
    };
    let n = cx.m.g.bricks.len();
    let mut v: Vec<usize> = ed
        .hot
        .get()
        .into_iter()
        .chain(sel)
        .filter(|&i| i < n && !cx.m.g.bricks[i].ghost)
        .collect();
    v.dedup();
    v
}

/// The rubber band and its targets while a port is dragged.
fn band(cx: &Cx, c: &Connect, p: &[Rect]) -> View {
    let Some(a) = cx.brick(&c.from) else {
        return View::default();
    };
    let (x1, y1) = (p[a].x + p[a].w, p[a].y + p[a].h / 2.0);
    let refused = cx.ed.check.with(|k| {
        k.as_ref()
            .filter(|(_, v)| v.redundant)
            .map(|(t, v)| (t.clone(), v.why.clone()))
    });
    let targets: Vec<View> = (0..cx.m.g.bricks.len().min(p.len()))
        .filter(|&i| i != a && !cx.m.g.bricks[i].ghost)
        .map(|i| {
            let r = p[i];
            let no = refused.as_ref().filter(|(t, _)| *t == cx.m.g.bricks[i].name);
            let cls = if no.is_some() { "ltarget no" } else { "ltarget" };
            let label = no.map(|_| view! { text(class="tlabel", x=r.x.to_string(), y=(r.y - 6.0).to_string()) { "already allowed" } });
            view! { rect(class=cls, x=(r.x - 3.0).to_string(), y=(r.y - 3.0).to_string(), width=(r.w + 6.0).to_string(), height=(r.h + 6.0).to_string(), rx="8") (label) }
        })
        .collect();
    let d = format!("M{x1:.1},{y1:.1} L{:.1},{:.1}", c.x, c.y);
    let (wx, wy) = (c.x.to_string(), (c.y - 10.0).to_string());
    let why = refused
        .filter(|(t, _)| t.starts_with("tag:"))
        .map(|(t, w)| {
            let text = format!("{t}: {w}");
            view! { text(class="tlabel", x=wx, y=wy) { (text) } }
        });
    let targets = View::from(targets);
    let bcls = format!("band l-{}", c.ty.name());
    view! {
        (targets)
        path(class=bcls, d=d)
        (why)
    }
}

/// The editor layer (empty outside edit mode).
pub fn layer(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    if !cx.editing() {
        return View::default();
    }
    let c = cx.clone();
    // The dragged brick only: a pan starting (mousedown) must not re-render the layer, or the
    // element under the pointer is replaced and its click (selecting a rule) is lost.
    let op = cx.op;
    let dragged = create_selector(move || match op.get() {
        Some(Op::Drag { b, .. }) => Some(b),
        _ => None,
    });
    let body = move || {
        let p = pos.get_clone();
        let spots = badge_spots(&c, &p);
        let ports: Vec<View> = ported(&c)
            .into_iter()
            .filter(|&b| b < p.len() && c.ed.connect.with(Option::is_none))
            .map(|b| ports(&c, b, p[b]))
            .collect();
        // Above the ports: a target's port strip can sit right where the arrowhead ends.
        let heads = super::retarget::handles(&c, &p);
        let band = c.ed.connect.get_clone().map(|k| band(&c, &k, &p));
        let d = dragged.get();
        let mv = c.ed.target.get_clone().map(|t| move_mark(&c, &t, &p, d));
        let (zone, lbl) = (new_lane_hint(&c, &p, d), drag_label(&c, d, &p));
        let (ports, badges) = (View::from(ports), badges(&spots));
        view! {
            g(class="edlayer") { (badges) (ports) (heads) (band) (zone) (mv) (lbl) }
        }
    };
    view! { (body) }
}
