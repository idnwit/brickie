//! State shared by the page components: the immutable model (graph, layout, diagnostics), the
//! signals (URL params, hover, pan/zoom, drag, popup) and the memoised evaluation.

use super::{Eval, evaluate};
use crate::layout::{self, Layout, Rect};
use crate::model::*;
use crate::platform;
use crate::state::*;
use std::collections::BTreeMap;
use std::rc::Rc;
use sycamore::prelude::*;

pub type Click = web_sys::MouseEvent;

/// Plain left click (no new-tab or download modifiers).
pub fn plain(e: &Click) -> bool {
    e.button() == 0 && !(e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key())
}

/// Immutable data of the current level plus its layout (computed once from its full edge set).
pub struct Model {
    pub cache: CacheInfo,
    /// Every level (breadcrumb, peek) and the index of the current one.
    pub levels: Rc<Vec<LevelView>>,
    pub lv: usize,
    pub g: GraphView,
    pub layout: Layout,
    pub diags: Vec<DiagView>,
    pub cards: Rc<Vec<Card>>,
    pub fns: Vec<FnRow>,
}

/// `S.view`: pan and zoom of the canvas, in stage pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vw {
    pub x: f64,
    pub y: f64,
    pub k: f64,
}

/// The pointer operation in progress on the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Pan {
        sx: f64,
        sy: f64,
        vx: f64,
        vy: f64,
    },
    Drag {
        b: usize,
        sx: f64,
        sy: f64,
        dx0: f64,
        dy0: f64,
    },
    Box {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    },
}

#[derive(Clone)]
pub struct Cx {
    pub params: Signal<Params>,
    /// The finding (card index) hovered in the left list.
    pub hover: Signal<Option<usize>>,
    pub modal: Signal<Option<String>>,
    /// Id of the open dropdown menu.
    pub menu: Signal<String>,
    /// Text filter of the findings list.
    pub q: Signal<String>,
    pub drag: Signal<BTreeMap<usize, (f64, f64)>>,
    pub view: Signal<Vw>,
    /// Size of `#stage` in pixels (browser only).
    pub stage: Signal<Option<(f64, f64)>>,
    pub op: Signal<Option<Op>>,
    /// A brick drag moved: the click that ends it must not select.
    pub moved: Signal<bool>,
    pub m: Rc<Model>,
    pub eval: ReadSignal<Eval>,
    /// Per brick score (open findings and complexity excess), follows accept/unaccept.
    pub scores: ReadSignal<Vec<BrickScore>>,
    /// Entry transition of the canvas: `zin`, `zout` or none.
    pub anim: &'static str,
}

/// The layout input of a page: bricks with their row, and one edge per (from, to, kind).
pub fn layout_input(g: &GraphView) -> layout::Input {
    layout::Input {
        bricks: g
            .bricks
            .iter()
            .map(|b| layout::InBrick {
                id: b.name.clone(),
                row: b.row,
            })
            .collect(),
        rows: g.rows.clone(),
        edges: (0..g.edges.len())
            .map(|i| layout::InEdge {
                id: edge_key(g, i),
                a: g.edges[i].from,
                b: g.edges[i].to,
            })
            .collect(),
    }
}

impl Cx {
    /// The state of level `lv`: shared signals from `sh`, fresh pan/zoom/drag and layout.
    pub fn new(sh: &Shared, lv: usize) -> Cx {
        let params = sh.params;
        let level = &sh.levels[lv];
        let layout = layout::layout(&layout_input(&level.graph));
        let m = Rc::new(Model {
            cache: sh.cache.clone(),
            levels: sh.levels.clone(),
            lv,
            g: level.graph.clone(),
            layout,
            diags: sh.diags.clone(),
            cards: sh.cards.clone(),
            fns: sh.fns.clone(),
        });
        let stage = create_signal(platform::stage_rect().map(|r| (r.2, r.3)));
        let view = create_signal(match stage.get() {
            Some(s) => fit_view(&m.layout, s),
            None => Vw {
                x: 0.0,
                y: 0.0,
                k: 1.0,
            },
        });
        let eval = {
            let m = m.clone();
            create_memo(move || {
                let acc: Vec<bool> = m.cards.iter().map(|c| c.accepted.get()).collect();
                params.with(|p| evaluate(&m.g, &m.diags, &acc, p))
            })
        };
        let scores = {
            let m = m.clone();
            create_memo(move || {
                let acc: Vec<bool> = m.cards.iter().map(|c| c.accepted.get()).collect();
                brick_scores(&m.g, &m.diags, &acc)
            })
        };
        Cx {
            params,
            hover: sh.hover,
            modal: sh.modal,
            menu: sh.menu,
            q: sh.q,
            drag: create_signal(BTreeMap::new()),
            view,
            stage,
            op: create_signal(None),
            moved: create_signal(false),
            m,
            eval,
            scores,
            anim: "",
        }
    }

    /// Change the params and the URL.
    pub fn go(&self, p: Params) {
        platform::push_url(&p.url());
        self.params.set(p);
    }

    /// The current level.
    pub fn level(&self) -> &LevelView {
        &self.m.levels[self.m.lv]
    }

    /// Brick index by name.
    pub fn brick(&self, name: &str) -> Option<usize> {
        self.m.g.bricks.iter().position(|b| b.name == name)
    }

    /// Select (or clear) through params, dropping a selected finding.
    pub fn select(&self, s: Sel) {
        let p = s.apply(&self.params.get_clone(), &self.m.g);
        self.go(p);
    }
}

/// `fitView`: the whole layout inside the stage, zoom between 0.15 and 1.4.
pub fn fit_view(l: &Layout, (sw, sh): (f64, f64)) -> Vw {
    let k = ((sw - 20.0) / l.w).min((sh - 20.0) / l.h).clamp(0.15, 1.4);
    Vw {
        x: (sw - l.w * k) / 2.0,
        y: ((sh - l.h * k) / 2.0).max(10.0),
        k,
    }
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

/// A link that changes the params: a real `href` for no-JS, a signal update when hydrated.
pub fn nav_link(
    cx: &Cx,
    label: String,
    class: &'static str,
    active: impl Fn(&Params) -> bool + 'static,
    change: impl Fn(&Params) -> Params + Clone + 'static,
) -> View {
    let on_click = change.clone();
    let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
    let href = move || change(&c1.params.get_clone()).url();
    let cls = move || {
        if active(&c2.params.get_clone()) {
            format!("{class} on")
        } else {
            class.to_string()
        }
    };
    view! {
        a(href=href, class=cls, on:click=move |e: Click| {
            if plain(&e) {
                e.prevent_default();
                let p = on_click(&c3.params.get_clone());
                c3.go(p);
            }
        }) { (label) }
    }
}

/// A link to a source location: opens the popup when hydrated, the source page otherwise.
pub fn src_link(
    modal: Signal<Option<String>>,
    href: String,
    text: String,
    class: &'static str,
) -> View {
    let target = href.clone();
    view! {
        a(href=href, class=class, target="_blank", on:click=move |e: Click| {
            if plain(&e) {
                e.prevent_default();
                modal.set(Some(target.clone()));
            }
        }) { (text) }
    }
}
