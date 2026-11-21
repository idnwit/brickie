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

/// The frame box and its caption: `web · exports [api] · crates/web/brickie.toml`.
pub fn frame_view(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> View {
    let lv = cx.level();
    // The config file by its last two components; the tooltip has the full path.
    let parts: Vec<&str> = lv.file.rsplit('/').take(2).collect();
    let file: String = parts.into_iter().rev().collect::<Vec<_>>().join("/");
    let full_label = if lv.path == "root" {
        format!("workspace \u{00B7} {file}")
    } else if lv.exports.len() <= 2 {
        format!(
            "{} \u{00B7} exports [{}] \u{00B7} {file}",
            lv.name(),
            lv.exports.join(", ")
        )
    } else {
        format!(
            "{} \u{00B7} {} exports \u{00B7} {file}",
            lv.name(),
            lv.exports.len()
        )
    };
    let tip = if lv.exports.is_empty() {
        format!("{} ({})", lv.name(), lv.file)
    } else {
        format!(
            "{} \u{00B7} exports [{}] ({})",
            lv.name(),
            lv.exports.join(", "),
            lv.file
        )
    };
    // Counter the zoom so the caption stays about 12 px on screen.
    let view = cx.view;
    let size = move || format!("font-size:{:.1}px", (12.0 / view.get().k).clamp(10.0, 26.0));
    let c = cx.clone();
    let fr: ReadSignal<Rect> =
        create_memo(move || pos.with(|p| frame_rect(&c.m.g, p)).unwrap_or_default());
    let rect = move || fr.get();
    let compact = (lv.path != "root").then(|| {
        if lv.exports.is_empty() {
            lv.name().to_string()
        } else {
            format!("{} \u{00B7} {} exp", lv.name(), lv.exports.len())
        }
    });
    let label = move || {
        let max_chars = (((rect().w - 16.0) / 7.2).floor() as usize).max(16);
        if full_label.chars().count() <= max_chars {
            full_label.clone()
        } else if let Some(ref c) = compact {
            if c.chars().count() <= max_chars {
                c.clone()
            } else {
                let head: String = c.chars().take(max_chars.saturating_sub(1)).collect();
                format!("{}\u{2026}", head.trim_end())
            }
        } else {
            let head: String = full_label
                .chars()
                .take(max_chars.saturating_sub(1))
                .collect();
            format!("{}\u{2026}", head.trim_end())
        }
    };
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
