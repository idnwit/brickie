//! What the canvas draws for each edge: its path data and the anchor of its label and mark.

use super::boundary::frame_route;
use super::frame::{cut, frame_rect};
use crate::layout::{Layout, Rect};
use crate::state::*;
use brickie_web_api::GraphView;
use sycamore::prelude::*;

/// Path data and label anchor of every edge; follows the box positions (drags) only.
pub type Routes = ReadSignal<Vec<Option<(String, f64, f64)>>>;

/// The memo of all edge routes for the live box positions `pos`.
pub fn routes(cx: &Cx, pos: ReadSignal<Vec<Rect>>) -> Routes {
    let cx = cx.clone();
    create_memo(move || {
        let p = pos.get_clone();
        let frame = frame_rect(&cx.m.g, &p);
        (0..cx.m.g.edges.len())
            .map(|e| edge_route(&cx.m.g, &cx.m.layout, e, &p, frame.as_ref()))
            .collect()
    })
}

/// Path data and anchor `(d, x, y)` of edge `e` for box positions `p` and the level's frame:
/// `None` for self edges, and for a frame edge when there is no frame. A private edge from
/// outside stops at the frame wall (mark at the wall); any other has its mark at the middle.
fn edge_route(
    g: &GraphView,
    layout: &Layout,
    e: usize,
    p: &[Rect],
    frame: Option<&Rect>,
) -> Option<(String, f64, f64)> {
    let r = if g.edges[e].frame() {
        frame_route(g, e, p, frame?)?
    } else {
        layout.path(e, p)?
    };
    let wall = frame.filter(|_| g.edges[e].wall).and_then(|f| cut(&r, f));
    if let Some((d, w)) = wall {
        return Some((d, w.x, w.y));
    }
    let m = r.mid();
    Some((r.d(), m.x, m.y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{InBrick, InEdge, Input, layout};
    use brickie_web_api::{BrickView, EdgeView};

    /// Bricks 0 (row 0) and 1 (row 1) joined by edge 0 (`1 -> 0`), plus a self edge 1.
    fn fixture(wall: bool) -> (GraphView, Layout) {
        let inp = Input {
            bricks: vec![
                InBrick {
                    id: "a".into(),
                    row: 0,
                },
                InBrick {
                    id: "b".into(),
                    row: 1,
                },
            ],
            rows: vec!["core".into(), "app".into()],
            edges: vec![
                InEdge {
                    id: "e0".into(),
                    a: 1,
                    b: 0,
                },
                InEdge {
                    id: "e1".into(),
                    a: 1,
                    b: 1,
                },
            ],
        };
        let edge = |from, to, wall| EdgeView {
            from,
            to,
            kind: "uses".into(),
            wall,
            ..Default::default()
        };
        let g = GraphView {
            rows: vec!["core".into(), "app".into()],
            bricks: vec![BrickView::default(), BrickView::default()],
            edges: vec![edge(1, 0, wall), edge(1, 1, false)],
        };
        (g, layout(&inp))
    }

    #[test]
    fn plain_edge_marks_the_middle_and_self_edge_has_no_route() {
        let (g, l) = fixture(false);
        let p = l.pos.clone();
        let (d, x, y) = edge_route(&g, &l, 0, &p, None).unwrap();
        let r = l.path(0, &p).unwrap();
        assert_eq!((d, x, y), (r.d(), r.mid().x, r.mid().y));
        assert!(edge_route(&g, &l, 1, &p, None).is_none());
    }

    #[test]
    fn wall_edge_stops_where_it_enters_the_frame() {
        let (g, l) = fixture(true);
        let p = l.pos.clone();
        let full = edge_route(&g, &l, 0, &p, None).unwrap();
        // The frame around the lower brick: the route from it cuts at once.
        let f = p[1];
        let (d, x, y) = edge_route(&g, &l, 0, &p, Some(&f)).unwrap();
        assert!(d.len() < full.0.len() && d.starts_with('M'));
        assert!(x >= f.x && x <= f.x + f.w && y >= f.y && y <= f.y + f.h);
        // A frame the route never enters leaves it whole.
        let far = Rect {
            x: -1e6,
            y: -1e6,
            w: 1.0,
            h: 1.0,
        };
        assert_eq!(edge_route(&g, &l, 0, &p, Some(&far)).unwrap(), full);
    }

    #[test]
    fn frame_edge_needs_a_frame() {
        let (mut g, l) = fixture(false);
        g.edges[0].under = vec![0];
        let p = l.pos.clone();
        assert!(edge_route(&g, &l, 0, &p, None).is_none());
    }
}
