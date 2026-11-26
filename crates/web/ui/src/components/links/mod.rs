//! `[[link]]`s on the canvas, one line per (from, to) brick pair, dashed like the edge kind.
//! View mode: the planned links (no code uses them yet), in the warning colour with a `!`.
//! Design mode: every link of the level, by kind colour (planned ones still carry the `!`); a
//! link no rule allows is red with a `\u{2715}` (the tooltip says why), one on a cycle of links
//! carries `\u{21BB}`, an agent's link `A`. The reason shows as a label on hover or selection only. In the editor a click
//! selects the link; in view mode it picks it (a tap on touch).

mod pair;
mod spot;

pub use pair::end;

use crate::components::canvas::{frame_rect, wall_route};
use crate::layout::{Pt, Rect, Route};
use crate::state::*;
use brickie_web_api::LevelLink;
use brickie_web_api::design::Op as DOp;
use pair::{BoundaryPair, Pair, boundary_pairs, pairs};
use spot::mark_spot;
use sycamore::prelude::*;

/// Pick key distinguishing local sibling pairs from projected boundary pairs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PickKey {
    Local(usize, usize),
    Boundary(usize, bool, Option<usize>),
}

/// Horizontal step between the marks of one link (world units).
const SLOT: f64 = 18.0;

/// Whether an agent's version since your last edit added link `e`.
fn by_agent(cx: &Cx, e: &LevelLink) -> bool {
    let Some(d) = cx.m.design.as_deref() else {
        return false;
    };
    agent_ops(&d.history).into_iter().any(|o| {
        matches!(o, DOp::AddLink { file, link } if *file == e.file && link.from == e.from && link.to == e.to)
    })
}

/// The route of pair `a -> b`: the routed layout path (around the boxes, from side ports), or a
/// plain cubic from the right to the left side when the pair is not routed.
fn path_of(cx: &Cx, routed: &[Option<Route>], a: usize, b: usize, p: &[Rect]) -> Route {
    if let Some(i) = cx.m.unused.iter().position(|&x| x == (a, b))
        && let Some(r) = routed.get(cx.m.plan0 + i).and_then(Clone::clone)
    {
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

/// Color key of a link's line and markers (`warn` in view mode, else `deny` / `agent` / kind).
fn link_tone(ty: LinkType, editing: bool, denied: bool, agent: bool) -> &'static str {
    if !editing {
        "warn"
    } else if denied {
        "deny"
    } else if agent {
        "agent"
    } else {
        ty.name()
    }
}

/// The arrowhead marker (`from -> to`) matching the link's kind shape and active tone.
fn marker_end(ty: LinkType, tone: &str) -> String {
    format!("url(#m-l-{}-{tone})", ty.name())
}

/// The start diamond marker for `reexports` links, empty for other kinds.
fn marker_start(ty: LinkType, tone: &str) -> String {
    if ty == LinkType::Reexports {
        format!("url(#t-l-reexports-{tone})")
    } else {
        String::new()
    }
}

/// The dash of the pair's edge kind (`implements`, `reexports`); `None` keeps the default dash.
fn kind_dash(e: &LevelLink) -> Option<&'static str> {
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

/// The marks of one link (`!` planned in view mode, denied, cycle, `A`) in a row from `at`, and the hover label after them.
fn marks_of(pr: &Pair, at: Pt, agent: bool, editing: bool) -> (Vec<View>, Option<View>) {
    // `!`: planned (no code uses the link yet) — shown in view mode; in design mode every drafted
    // link is expected to be planned, so only show actual issues (`✕` denied, `↻` cycle).
    let mut glyphs = if !editing && pr.planned() {
        vec!["!"]
    } else {
        vec![]
    };
    if pr.denied() {
        glyphs.push("\u{2715}");
    }
    if pr.cycle() {
        glyphs.push("\u{21BB}");
    }
    let (ax, ay) = (at.x, at.y);
    let ty = (ay + 4.0).to_string();
    let mut x = ax;
    let mut shown: Vec<View> = Vec::new();
    for g in glyphs {
        shown.push(glyph(x, ay, g));
        x += SLOT;
    }
    if agent {
        let bx = x.to_string();
        shown.push(
            view! { text(class="abadge", x=bx, y=ty.clone(), "text-anchor"="middle") { "A" } },
        );
        x += SLOT;
    }
    let lx = if x > at.x {
        x - SLOT + 14.0
    } else {
        at.x + 10.0
    };
    let ly = (at.y + 4.0).to_string();
    let label = pr.label().map(|t| {
        view! { text(class="rlabel", x=lx.to_string(), y=ly) { (t) } }
    });
    (shown, label)
}

/// Select the link after the selected one among `refs` (the first when none is selected), and
/// open the inspector.
fn select_next(ed: Edit, refs: &[RuleRef]) {
    // A pair of several links: each click selects the next one.
    let at = ed
        .rule
        .with_untracked(|x| x.as_ref().and_then(|r| refs.iter().position(|q| q == r)));
    ed.rule
        .set(refs.get(at.map_or(0, |i| (i + 1) % refs.len())).cloned());
    ed.tab.set("inspect");
}

/// Per-route display metadata passed to [`draw_route`].
struct RouteDraw {
    route: Route,
    picked: Signal<Option<PickKey>>,
    key: PickKey,
    tip: String,
    projected: bool,
}

fn route_flags(pr: &Pair, projected: bool) -> String {
    format!(
        "{}{}{}",
        if pr.denied() { " denied" } else { "" },
        if pr.cycle() { " cycle" } else { "" },
        if projected { " projected" } else { "" }
    )
}

fn kind_label_view(editing: bool, ktext: String, at: Pt) -> Option<View> {
    (editing && ktext != "any").then(|| {
        let kx = at.x.to_string();
        let ky = (at.y - 10.0).to_string();
        view! { text(class="klabel", x=kx, y=ky, "text-anchor"="middle") { (ktext) } }
    })
}

/// One link pair along `draw.route`: line, hit area, marks, kind label, and hover reason.
fn draw_route(cx: &Cx, pr: &Pair, p: &[Rect], marks: &mut Vec<Pt>, draw: RouteDraw) -> View {
    let RouteDraw {
        route,
        picked,
        key,
        tip,
        projected,
    } = draw;
    let lead = pr.lead();
    let ty = LinkType::of_kinds(&lead.kinds);
    let flags = route_flags(pr, projected);
    let agent = pr.rules.iter().any(|e| by_agent(cx, e));
    let d = route.d();
    let at = mark_spot(&route, p, marks);
    let (ed, editing) = (cx.ed, cx.editing());
    let (shown, label) = marks_of(pr, at, agent, editing);
    let refs: Vec<RuleRef> = pr.rules.iter().map(|e| RuleRef::served(e)).collect();
    let tone = link_tone(ty, editing, pr.denied(), agent);
    let (m_end, m_start) = (marker_end(ty, tone), marker_start(ty, tone));
    let cap = if ty == LinkType::Reexports {
        "round"
    } else {
        "butt"
    };
    let klabel = kind_label_view(editing, pr.kind_label(), at);
    let (sel_refs, dash) = (refs.clone(), kind_dash(lead));
    let cls = move || -> String {
        let sel = editing
            && !projected
            && ed
                .rule
                .with(|x| x.as_ref().is_some_and(|r| sel_refs.contains(r)));
        let pick = picked.get() == Some(key);
        let agent = if agent { " agent" } else { "" };
        let sel = if sel { " sel" } else { "" };
        let pick = if pick { " picked" } else { "" };
        format!("pedge plink l-{}{flags}{agent}{sel}{pick}", ty.name())
    };
    let click = move |ev: Click| {
        ev.stop_propagation();
        picked.set((picked.get_untracked() != Some(key)).then_some(key));
        if editing && !projected {
            select_next(ed, &refs);
        }
    };
    let dash_style = dash.map_or_else(String::new, |s| format!("stroke-dasharray:{s}"));
    let d2 = d.clone();
    let shown = View::from(shown);
    view! {
        g(class=cls, on:click=click) {
            svg_title { (tip) }
            path(class="hit", d=d2)
            path(class="line", d=d, style=dash_style, "stroke-linecap"=cap, "marker-end"=m_end, "marker-start"=m_start)
            (shown)
            (klabel)
            (label)
        }
    }
}

/// Route of a projected boundary pair: directly to its target child, or to the nearest frame wall.
fn boundary_route(
    cx: &Cx,
    routed: &[Option<Route>],
    bp: &BoundaryPair,
    all: &[BoundaryPair],
    p: &[Rect],
    frame: Option<&Rect>,
) -> Option<Route> {
    if let Some(c) = bp.child {
        if bp.ghost >= p.len() || c >= p.len() {
            return None;
        }
        let (a, b) = if bp.inbound {
            (bp.ghost, c)
        } else {
            (c, bp.ghost)
        };
        return Some(path_of(cx, routed, a, b, p));
    }
    let f = frame?;
    let n = p.get(bp.ghost)?;
    let same: Vec<usize> = (0..all.len())
        .filter(|&i| all[i].child.is_none() && all[i].ghost == bp.ghost)
        .collect();
    let k = same
        .iter()
        .position(|&i| {
            all[i].inbound == bp.inbound && all[i].ghost == bp.ghost && all[i].child.is_none()
        })
        .unwrap_or(0) as f64;
    let off = (k - (same.len() as f64 - 1.0) / 2.0) * 16.0;
    Some(wall_route(n, f, bp.inbound, off))
}

/// The links layer of the canvas (inside the pan/zoom group).
pub fn layer(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    let c = cx.clone();
    let picked: Signal<Option<PickKey>> = create_signal(None);
    let layer_cls = if cx.editing() {
        "plinks editing"
    } else {
        "plinks view"
    };
    let body = move || {
        let p = pos.get_clone();
        let routed = c.m.layout.rerouted_at(&p);
        let mut marks = Vec::new();
        let mut items: Vec<View> = pairs(&c)
            .iter()
            .filter(|pr| pr.a < p.len() && pr.b < p.len())
            .map(|pr| {
                let draw = RouteDraw {
                    route: path_of(&c, &routed, pr.a, pr.b, &p),
                    picked,
                    key: PickKey::Local(pr.a, pr.b),
                    tip: pr.tip(),
                    projected: false,
                };
                draw_route(&c, pr, &p, &mut marks, draw)
            })
            .collect();
        let frame = frame_rect(&c.m.g, &p);
        let b_pairs = boundary_pairs(&c);
        for bp in &b_pairs {
            if let Some(route) = boundary_route(&c, &routed, bp, &b_pairs, &p, frame.as_ref()) {
                let draw = RouteDraw {
                    route,
                    picked,
                    key: PickKey::Boundary(bp.ghost, bp.inbound, bp.child),
                    tip: bp.tip(),
                    projected: true,
                };
                items.push(draw_route(&c, &bp.pair, &p, &mut marks, draw));
            }
        }
        let items = View::from(items);
        view! { g(class=layer_cls) { (items) } }
    };
    view! { (body) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_tone_and_markers_follow_mode_and_kind() {
        assert_eq!(link_tone(LinkType::Uses, false, false, false), "warn");
        assert_eq!(link_tone(LinkType::Implements, false, true, true), "warn");
        assert_eq!(link_tone(LinkType::Uses, true, false, false), "uses");
        assert_eq!(
            link_tone(LinkType::Implements, true, false, false),
            "implements"
        );
        assert_eq!(link_tone(LinkType::Reexports, true, false, true), "agent");
        assert_eq!(link_tone(LinkType::Reexports, true, true, true), "deny");

        assert_eq!(marker_end(LinkType::Uses, "uses"), "url(#m-l-uses-uses)");
        assert_eq!(marker_start(LinkType::Uses, "uses"), "");
        assert_eq!(
            marker_end(LinkType::Implements, "implements"),
            "url(#m-l-implements-implements)"
        );
        assert_eq!(
            marker_end(LinkType::Reexports, "reexports"),
            "url(#m-l-reexports-reexports)"
        );
        assert_eq!(
            marker_start(LinkType::Reexports, "reexports"),
            "url(#t-l-reexports-reexports)"
        );
        assert_eq!(
            marker_start(LinkType::Reexports, "warn"),
            "url(#t-l-reexports-warn)"
        );
    }
}
