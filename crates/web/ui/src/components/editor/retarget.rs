//! Retargeting a link: the selected link shows a handle on its arrowhead; dragging it draws
//! the rubber band from the link's source brick, and dropping on another sibling changes the
//! link's `to` (the link is replaced: one version, one undo step). Kinds, scope and reason
//! stay as they were.

use crate::components::links::end;
use crate::layout::Rect;
use crate::state::*;
use sycamore::prelude::*;

/// The arrowhead handle of the selected link.
pub fn handles(cx: &Cx, p: &[Rect]) -> View {
    let Some(r) = cx.ed.rule.get_clone() else {
        return View::default();
    };
    let Some(e) = (cx.level().links.iter()).find(|e| RuleRef::served(e) == r) else {
        return View::default();
    };
    let ty = LinkType::of_kinds(&e.kinds);
    let (Some(a), Some(b)) = (end(cx, &e.from), end(cx, &e.to)) else {
        return View::default();
    };
    if a == b || a >= p.len() || b >= p.len() {
        return View::default();
    }
    let at = (cx
        .unused_route(a, b, p)
        .and_then(|q| q.segs.last().map(|s| s[3])))
    .map_or((p[b].x, p[b].y + p[b].h / 2.0), |q| (q.x, q.y));
    handle(cx, &r, a, ty, at)
}

/// The arrowhead handle of link `r` drawn from brick `from` (index), at world point `at`.
fn handle(cx: &Cx, r: &RuleRef, from: usize, ty: LinkType, at: (f64, f64)) -> View {
    let ed = cx.ed;
    let (name, r) = (cx.m.g.bricks[from].name.clone(), r.clone());
    let down = move |e: Click| {
        e.stop_propagation();
        e.prevent_default();
        ed.check.set(None);
        ed.connect.set(Some(Connect {
            from: name.clone(),
            ty,
            retarget: Some(r.clone()),
            x: at.0,
            y: at.1,
        }));
    };
    view! {
        g(class=format!("rhandle l-{}", ty.name()), "data-retarget"="1", on:mousedown=down) {
            svg_title { "drag onto another brick to change the target" }
            circle(cx=at.0.to_string(), cy=at.1.to_string(), r="7")
        }
    }
}
