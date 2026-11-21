//! Pan/zoom of the canvas: the view, fitting a layout into the stage, dragged box positions.

use super::NEW_LANE_ROOM;
use crate::layout::{Layout, Rect};
use std::collections::BTreeMap;

/// `S.view`: pan and zoom of the canvas, in stage pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vw {
    pub x: f64,
    pub y: f64,
    pub k: f64,
}

/// `fitView`: the whole layout inside the stage, zoom between 0.15 and 1.4.
/// `room`: world height kept free under the layout (edit mode: the new-lane zone).
pub(super) fn fit_view(l: &Layout, (sw, sh): (f64, f64), room: f64) -> Vw {
    let top = if room > 0.0 { 88.0 } else { 44.0 };
    let h = l.h + room;
    let avail_h = (sh - top - 10.0).max(80.0);
    let k = ((sw - 20.0) / l.w).min(avail_h / h).clamp(0.15, 1.4);
    Vw {
        x: (sw - l.w * k) / 2.0,
        y: (top + (avail_h - h * k) / 2.0).max(top),
        k,
    }
}

/// The room [`fit_view`] keeps under the layout.
pub(super) fn fit_room(editing: bool) -> f64 {
    if editing { NEW_LANE_ROOM } else { 0.0 }
}

/// Box positions with the dragged offsets applied (`layoutFor`'s `P`).
pub fn positions(l: &Layout, drag: &BTreeMap<usize, (f64, f64)>) -> Vec<Rect> {
    let mut p = l.pos.clone();
    for (b, (dx, dy)) in drag {
        if let Some(r) = p.get_mut(*b) {
            r.x += dx;
            r.y += dy;
        }
    }
    p
}
