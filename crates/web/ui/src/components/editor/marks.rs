//! Drop-target marks of the editor layer: the folder brick (charging while a hold would open
//! it), the lane band, the new-lane zone, the frame (move up) and refused bricks, plus the
//! result label under a dragged brick.

use crate::layout::Rect;
use crate::state::*;
use sycamore::prelude::*;

pub(super) fn mark(r: Rect, cls: &'static str, label: String) -> View {
    view! {
        rect(class=cls, x=r.x.to_string(), y=r.y.to_string(), width=r.w.to_string(), height=r.h.to_string(), rx="8")
        text(class="tlabel", x=(r.x + 8.0).to_string(), y=(r.y - 6.0).to_string()) { (label) }
    }
}

/// The drop target of a drag: the folder brick, the lane band, the new-lane zone or the frame
/// (move up), each with what the drop does; a refused brick in red.
pub(super) fn move_mark(cx: &Cx, t: &MoveTarget, p: &[Rect], dragged: Option<usize>) -> View {
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
            let spring = b.file.is_some() && cx.ed.spring.with(Option::is_some);
            let (cls, what) = match spring {
                true => (
                    "mtarget folder spring",
                    format!("{what} \u{00B7} hold to open"),
                ),
                false => ("mtarget folder", what),
            };
            let (body, tab) = (
                mark(p[*i], cls, what),
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
        MoveTarget::NewLane => match new_lane_zone(rows, &cx.m.g.bricks, p, dragged) {
            Some(r) => mark(r, "mtarget lane new", "+ new tag lane".into()),
            None => View::default(),
        },
        MoveTarget::Up(l) => mark(frame, "mtarget up", format!("\u{2191} move to {l}")),
        MoveTarget::No(why) => match cx.ed.over.get() {
            Some(i) if i < n => mark(p[i], "mtarget no", format!("\u{2715} {why}")),
            _ => View::default(),
        },
        MoveTarget::Into(_) => View::default(),
    }
}

/// While a drag is on: the "+ new tag lane" zone, faint, so it can be found.
pub(super) fn new_lane_hint(cx: &Cx, p: &[Rect], dragged: Option<usize>) -> View {
    let dragging = cx.ed.carry.with(Option::is_some) || !cx.ed.aim_label.with(String::is_empty);
    let zone = new_lane_zone(&cx.m.layout.rows, &cx.m.g.bricks, p, dragged);
    let Some(r) = zone.filter(|_| dragging) else {
        return View::default();
    };
    view! {
        rect(class="newlane", x=r.x.to_string(), y=r.y.to_string(), width=r.w.to_string(), height=r.h.to_string(), rx="8")
        text(class="newlanet", x=(r.x + r.w / 2.0).to_string(), y=(r.y + r.h / 2.0 + 4.0).to_string()) { "+ new tag lane" }
    }
}

/// The result label above a dragged brick (a tile's label rides on its ghost).
pub(super) fn drag_label(cx: &Cx, dragged: Option<usize>, p: &[Rect]) -> View {
    let Some(b) = dragged else {
        return View::default();
    };
    let label = cx.ed.aim_label.get_clone();
    if label.is_empty() || b >= p.len() {
        return View::default();
    }
    let r = p[b];
    view! { text(class="tlabel aim", x=r.x.to_string(), y=(r.y + r.h + 16.0).to_string()) { (label) } }
}
