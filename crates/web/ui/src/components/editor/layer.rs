//! The editor's layer of the canvas (inside the pan/zoom group): planned rule edges (dashed per
//! type, deny red, agent violet, "?" without a reason), agent badges, typed ports on the hot
//! brick, the rubber band of a port drag with its valid and refused targets, and the target
//! of a brick being moved.

use super::connect;
use crate::layout::Rect;
use crate::state::*;
use brickie_web_api::change::{By, PlannedEdge, State};
use sycamore::prelude::*;

fn ends(cx: &Cx, sel: &str) -> Vec<usize> {
    (0..cx.m.g.bricks.len())
        .filter(|&i| !cx.m.g.bricks[i].ghost && sel != "*" && names(sel, &cx.m.g.bricks[i]))
        .collect()
}

fn edge_class(e: &PlannedEdge) -> String {
    let ty = LinkType::of_rule(&e.kinds, &e.action);
    let agent = e.by == Some(By::Agent) && e.state == Some(State::Proposed);
    format!("pedge l-{}{}", ty.name(), if agent { " agent" } else { "" })
}

/// One planned rule: a dashed line per (from, to) pair of bricks its selectors name.
fn planned(cx: &Cx, e: &PlannedEdge, p: &[Rect]) -> View {
    let r = RuleRef {
        file: e.file.clone(),
        index: e.rule,
    };
    let ed = cx.ed;
    let sel = ed.rule.with(|x| x.as_ref() == Some(&r));
    let cls = format!("{}{}", edge_class(e), if sel { " sel" } else { "" });
    let agent = e.by == Some(By::Agent) && e.state == Some(State::Proposed);
    let mut out = Vec::new();
    for a in ends(cx, &e.from) {
        for b in ends(cx, &e.to).into_iter().filter(|&b| b != a) {
            let (x1, y1) = (p[a].x + p[a].w, p[a].y + p[a].h / 2.0);
            let (x2, y2) = (p[b].x, p[b].y + p[b].h / 2.0);
            let (mx, my) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
            let d = format!(
                "M{x1:.1},{y1:.1} C{:.1},{y1:.1} {:.1},{y2:.1} {x2:.1},{y2:.1}",
                mx, mx
            );
            let r = r.clone();
            let click = move |ev: Click| {
                ev.stop_propagation();
                ed.rule.set(Some(r.clone()));
                ed.tab.set("inspect");
            };
            let tip = format!(
                "{} \u{2192} {} ({}) {}",
                e.from,
                e.to,
                e.action,
                e.reason.clone().unwrap_or_else(|| "no reason yet".into())
            );
            let q = e.reason.is_none().then(|| view! { g(class="qmark") { circle(cx=mx.to_string(), cy=my.to_string(), r="8") text(x=mx.to_string(), y=(my + 4.0).to_string(), "text-anchor"="middle") { "?" } } });
            let a_badge = agent.then(|| view! { text(class="abadge", x=(mx + 10.0).to_string(), y=(my - 8.0).to_string()) { "A" } });
            let (cls, d2) = (cls.clone(), d.clone());
            out.push(view! {
                g(class=cls, on:click=click) {
                    svg_title { (tip) }
                    path(class="hit", d=d2)
                    path(class="line", d=d)
                    (q) (a_badge)
                }
            });
        }
    }
    View::from(out)
}

fn badges(cx: &Cx, p: &[Rect]) -> View {
    let Some(ch) = cx.m.change.as_deref() else {
        return View::default();
    };
    let lv = cx.level().path.clone();
    let file = ch.files.get(&lv);
    let out: Vec<View> = (0..cx.m.g.bricks.len().min(p.len()))
        .filter_map(|i| {
            let b = &cx.m.g.bricks[i];
            let touched = |r: &&brickie_web_api::change::OpRecord| Some(&r.file) == file && op_brick(&r.op) == Some(b.name.as_str());
            let conflict = ch.log.iter().filter(touched).any(|r| r.conflict.is_some() && r.state == State::Proposed);
            let agent = b.proposed || !proposals_on(ch, &lv, &b.name).is_empty();
            let (label, cls) = match (conflict, agent) {
                (true, _) => ("!", "ebadge conflict"),
                (_, true) => ("A", "ebadge agent"),
                _ => return None,
            };
            let (x, y) = (p[i].x - 6.0, p[i].y - 6.0);
            Some(view! { g(class=cls) { circle(cx=x.to_string(), cy=y.to_string(), r="9") text(x=x.to_string(), y=(y + 4.0).to_string(), "text-anchor"="middle") { (label) } } })
        })
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

fn mark(r: Rect, cls: &'static str, label: String) -> View {
    view! {
        rect(class=cls, x=r.x.to_string(), y=r.y.to_string(), width=r.w.to_string(), height=r.h.to_string(), rx="8")
        text(class="tlabel", x=(r.x + 8.0).to_string(), y=(r.y - 6.0).to_string()) { (label) }
    }
}

/// The drop target of a drag: the folder brick, the lane band, the new-lane zone or the frame
/// (move up), each with what the drop does; a refused brick in red.
fn move_mark(cx: &Cx, t: &MoveTarget, p: &[Rect]) -> View {
    let n = p.len().min(cx.m.g.bricks.len());
    let frame = crate::components::canvas::frame_rect(&cx.m.g, p).unwrap_or_default();
    let rows = &cx.m.layout.rows;
    match t {
        MoveTarget::Into(i) if *i < n => {
            let b = &cx.m.g.bricks[*i];
            let lid = Rect {
                x: p[*i].x + 6.0,
                y: p[*i].y - 7.0,
                w: 34.0,
                h: 9.0,
            };
            let what = if b.file.is_some() {
                format!("into {} ({} inside)", b.name, b.children.len())
            } else {
                format!("into {} (becomes a group)", b.name)
            };
            let (body, tab) = (
                mark(p[*i], "mtarget folder", what),
                mark(lid, "mtarget lid", String::new()),
            );
            view! { (tab) (body) }
        }
        MoveTarget::Lane(l) => match lane_rect(rows, &frame, l.as_deref()) {
            Some(r) => mark(
                r,
                "mtarget lane",
                format!(
                    "\u{2192} tag: {}",
                    l.as_deref().unwrap_or("none (untagged)")
                ),
            ),
            None => View::default(),
        },
        MoveTarget::NewLane => mark(
            new_lane_rect(rows, &frame),
            "mtarget lane new",
            "+ new tag lane".into(),
        ),
        MoveTarget::Up(l) => mark(frame, "mtarget up", format!("\u{2191} move to {l}")),
        MoveTarget::No(why) => match cx.ed.over.get() {
            Some(i) if i < n => mark(p[i], "mtarget no", format!("\u{2715} {why}")),
            _ => View::default(),
        },
        MoveTarget::Into(_) => View::default(),
    }
}

/// While a drag is on: the "+ new tag lane" zone, faint, so it can be found.
fn new_lane_hint(cx: &Cx, p: &[Rect]) -> View {
    let dragging = cx.ed.carry.get().is_some() || !cx.ed.aim_label.with(String::is_empty);
    let Some(f) = crate::components::canvas::frame_rect(&cx.m.g, p).filter(|_| dragging) else {
        return View::default();
    };
    let r = new_lane_rect(&cx.m.layout.rows, &f);
    view! {
        rect(class="newlane", x=r.x.to_string(), y=r.y.to_string(), width=r.w.to_string(), height=r.h.to_string(), rx="8")
        text(class="newlanet", x=(r.x + r.w / 2.0).to_string(), y=(r.y + r.h / 2.0 + 4.0).to_string()) { "+ new tag lane" }
    }
}

/// The result label above a dragged brick (a tile's label rides on its ghost).
fn drag_label(cx: &Cx, p: &[Rect]) -> View {
    let Some(Op::Drag { b, .. }) = cx.op.get() else {
        return View::default();
    };
    let label = cx.ed.aim_label.get_clone();
    if label.is_empty() || b >= p.len() {
        return View::default();
    }
    let r = p[b];
    view! { text(class="tlabel aim", x=r.x.to_string(), y=(r.y + r.h + 16.0).to_string()) { (label) } }
}

/// The editor layer (empty outside edit mode).
pub fn layer(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    if !cx.editing() {
        return View::default();
    }
    let c = cx.clone();
    let body = move || {
        let p = pos.get_clone();
        let edges: Vec<View> = c
            .level()
            .planned_edges
            .iter()
            .map(|e| planned(&c, e, &p))
            .collect();
        let ports: Vec<View> = ported(&c)
            .into_iter()
            .filter(|&b| b < p.len() && c.ed.connect.with(Option::is_none))
            .map(|b| ports(&c, b, p[b]))
            .collect();
        let band = c.ed.connect.get_clone().map(|k| band(&c, &k, &p));
        let mv = c.ed.target.get_clone().map(|t| move_mark(&c, &t, &p));
        let (zone, lbl) = (new_lane_hint(&c, &p), drag_label(&c, &p));
        let (edges, ports, badges) = (View::from(edges), View::from(ports), badges(&c, &p));
        view! {
            g(class="edlayer") { (edges) (badges) (ports) (band) (zone) (mv) (lbl) }
        }
    };
    view! { (body) }
}
