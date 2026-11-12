//! The frame of the current level: a box around its children (the faded outside neighbours
//! stay outside it) labelled with the level name, its exports and config file. A `deps/private`
//! edge reported at this level from outside is cut where it meets the frame wall.

use crate::layout::{Pt, Rect, Route};
use crate::state::*;
use brickie_web_api::GraphView;
use sycamore::prelude::*;

/// Space between the children and the frame (narrow at the sides: the row captions sit there).
const FPAD: f64 = 20.0;
const FPADX: f64 = 10.0;

/// The frame around the children of the level (`None` when the level has no children).
pub fn frame_rect(g: &GraphView, pos: &[Rect]) -> Option<Rect> {
    let inner: Vec<&Rect> = (0..g.bricks.len().min(pos.len()))
        .filter(|&i| !g.bricks[i].ghost)
        .map(|i| &pos[i])
        .collect();
    let x0 = inner.iter().map(|r| r.x).reduce(f64::min)?;
    let y0 = inner.iter().map(|r| r.y).reduce(f64::min)?;
    let x1 = inner.iter().map(|r| r.x + r.w).reduce(f64::max)?;
    let y1 = inner.iter().map(|r| r.y + r.h).reduce(f64::max)?;
    Some(Rect {
        x: x0 - FPADX,
        y: y0 - FPAD,
        w: x1 - x0 + 2.0 * FPADX,
        h: y1 - y0 + 2.0 * FPAD,
    })
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

/// The frame box and its caption: `web · exports [api] · crates/web/brickie.toml`.
pub fn frame_view(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    let lv = cx.level();
    // The config file by its last two components; the tooltip has the full path.
    let parts: Vec<&str> = lv.file.rsplit('/').take(2).collect();
    let file: String = parts.into_iter().rev().collect::<Vec<_>>().join("/");
    let label = if lv.path == "root" {
        format!("workspace \u{00B7} {file}")
    } else {
        format!(
            "{} \u{00B7} exports [{}] \u{00B7} {file}",
            lv.name(),
            lv.exports.join(", ")
        )
    };
    let tip = lv.file.clone();
    // Counter the zoom so the caption stays about 12 px on screen.
    let view = cx.view;
    let size = move || format!("font-size:{:.1}px", (12.0 / view.get().k).clamp(10.0, 26.0));
    let c = cx.clone();
    let fr: ReadSignal<Rect> =
        create_memo(move || pos.with(|p| frame_rect(&c.m.g, p)).unwrap_or_default());
    let rect = move || fr.get();
    let (x, y, w, h) = (
        move || rect().x.to_string(),
        move || rect().y.to_string(),
        move || rect().w.to_string(),
        move || rect().h.to_string(),
    );
    let (lx, ly) = (
        move || (rect().x + 8.0).to_string(),
        move || (rect().y - 8.0).to_string(),
    );
    view! {
        g(class="frame") {
            rect(class="fbox", x=x, y=y, width=w, height=h, rx="12")
            text(class="flab", x=lx, y=ly, style=size) { svg_title { (tip) } (label) }
        }
    }
}
