//! The frame of the current level: a box around its children (the faded outside neighbours
//! stay outside it) labelled with the level name, its exports and config file. A `deps/private`
//! edge reported at this level from outside is cut where it meets the frame wall.

use crate::layout::{Pt, Rect, Route};
use crate::state::*;
use brickie_web_api::GraphView;
use sycamore::prelude::*;

/// The frame around the children of the level (`None` when the level has no children).
pub fn frame_rect(g: &GraphView, pos: &[Rect]) -> Option<Rect> {
    frame_of(&g.bricks, pos, None)
}

fn inside(f: &Rect, p: &Pt) -> bool {
    p.x >= f.x && p.x <= f.x + f.w && p.y >= f.y && p.y <= f.y + f.h
}

/// The route up to where it first enters the frame: path data and the wall point.
pub fn cut(r: &Route, f: &Rect) -> Option<(String, Pt)> {
    let pts = r.sample(3.0);
    let at = pts.iter().position(|p| inside(f, p))?;
    let mut d = String::new();
    for (i, p) in pts[..=at].iter().enumerate() {
        d += &format!("{}{:.1},{:.1}", if i == 0 { "M" } else { " L" }, p.x, p.y);
    }
    Some((d, pts[at]))
}

/// Horizontal span and top/bottom divider Y coordinates separating boundary bricks (`ghost`)
/// from the level's inner bricks.
fn divider_geom(g: &GraphView, pos: &[Rect]) -> (f64, f64, f64, f64) {
    let f = frame_rect(g, pos).unwrap_or_default();
    let (mut x1, mut x2) = (f.x, f.x + f.w);
    for (b, r) in g.bricks.iter().zip(pos) {
        if b.ghost {
            x1 = x1.min(r.x);
            x2 = x2.max(r.x + r.w);
        }
    }
    (x1, x2, f.y, f.y + f.h)
}

/// Thin horizontal divider lines above and below the inner bricks whenever a sub-level has
/// outside boundary bricks (`inbound` above / `outbound` below). Works in both `Outside -> frame`
/// (giving frame arrows a clean landing line) and `Outside -> children` modes.
pub fn frame_view(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    if cx.level().path == "root" {
        return View::default();
    }
    let Some(f0) = frame_rect(&cx.m.g, &cx.m.layout.pos) else {
        return View::default();
    };
    let has_in =
        cx.m.g
            .bricks
            .iter()
            .zip(&cx.m.layout.pos)
            .any(|(b, r)| b.ghost && r.y + r.h <= f0.y + 1.0);
    let has_out =
        cx.m.g
            .bricks
            .iter()
            .zip(&cx.m.layout.pos)
            .any(|(b, r)| b.ghost && r.y >= f0.y + f0.h - 1.0);
    if !has_in && !has_out {
        return View::default();
    }
    let c = cx.clone();
    let geom: ReadSignal<(f64, f64, f64, f64)> =
        create_memo(move || pos.with(|p| divider_geom(&c.m.g, p)));
    let top = has_in.then(|| {
        let (x1, x2, y) = (
            move || geom.get().0.to_string(),
            move || geom.get().1.to_string(),
            move || geom.get().2.to_string(),
        );
        view! { line(class="fline", x1=x1, y1=y, x2=x2, y2=y) }
    });
    let bot = has_out.then(|| {
        let (x1, x2, y) = (
            move || geom.get().0.to_string(),
            move || geom.get().1.to_string(),
            move || geom.get().3.to_string(),
        );
        view! { line(class="fline", x1=x1, y1=y, x2=x2, y2=y) }
    });
    view! {
        g(class="frame") {
            (top)
            (bot)
        }
    }
}
