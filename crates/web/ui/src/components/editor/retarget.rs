//! Retargeting a rule: the selected rule edge shows a handle on its arrowhead; dragging it
//! draws the rubber band from the rule's source brick, and dropping on another sibling or tag
//! lane changes the rule's `to` (the rule is replaced: one version, one undo step). Kinds, scope, action, reason and
//! help stay as they were.

use crate::components::links::ends;
use crate::layout::Rect;
use crate::state::*;
use sycamore::prelude::*;

/// The arrowhead handles of the selected planned rule, one per (from, to) pair it draws.
pub fn handles(cx: &Cx, p: &[Rect]) -> View {
    let Some(r) = cx.ed.rule.get_clone() else {
        return View::default();
    };
    let Some(e) =
        (cx.level().planned_edges.iter()).find(|e| e.file == r.file && planned_key(e) == r.key)
    else {
        return View::default();
    };
    let ty = LinkType::of_rule(&e.kinds, &e.action);
    let mut out = Vec::new();
    for a in ends(cx, &e.from).into_iter().filter(|&a| a < p.len()) {
        for b in ends(cx, &e.to)
            .into_iter()
            .filter(|&b| b != a && b < p.len())
        {
            let end = (cx
                .planned_route(a, b, p)
                .and_then(|q| q.segs.last().map(|s| s[3])))
            .map_or((p[b].x, p[b].y + p[b].h / 2.0), |q| (q.x, q.y));
            out.push(handle(cx, &r, a, ty, end));
        }
    }
    View::from(out)
}

/// The arrowhead handle of rule `r` drawn from brick `from` (index), at world point `at`.
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
            svg_title { "drag onto another brick or tag lane to change the target" }
            circle(cx=at.0.to_string(), cy=at.1.to_string(), r="7")
        }
    }
}
