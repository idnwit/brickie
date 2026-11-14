//! State shared by the page components: the immutable model (graph, layout, diagnostics), the
//! signals (URL params, hover, pan/zoom, drag, popup) and the memoised evaluation.

use super::{Eval, evaluate};
use crate::layout::{self, Layout, Rect};
use crate::state::*;
use brickie_web_api::*;
use std::collections::BTreeMap;
use std::rc::Rc;
use sycamore::prelude::*;

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
    /// The planned rule pairs routed by the layout, as layout edges
    /// `plan0 + i` (empty outside edit mode).
    pub planned: Vec<(usize, usize)>,
    pub plan0: usize,
    /// Edit mode: the design at its head.
    pub design: Option<Rc<DesignState>>,
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
    /// Fit the view once the stage is measured (false when a patch kept the view).
    pub fit: bool,
    pub live: Live,
    pub ed: Edit,
    pub nav: Signal<Nav>,
    pub bus: Bus,
}

/// The layout of `level`: automatic from its full edge set (without the frame edges, which are
/// appended after the original ones; with the planned rule pairs after them), then the editor's stored positions, then the positions a
/// patch keeps on screen.
fn level_layout(
    level: &LevelView,
    design: Option<&DesignState>,
    editing: bool,
    keep: Option<&Keep>,
) -> Layout {
    let extra = planned_pairs(level);
    let mut layout = layout::layout(&layout_input(&level.graph, &extra));
    // The editor's stored positions win over the automatic layout.
    if let Some(ch) = design.filter(|_| editing) {
        apply_hints(
            &mut layout.pos,
            &level.graph.bricks,
            &ch.layout,
            &level.path,
        );
    }
    if let Some(k) = keep {
        restore_positions(&mut layout.pos, &level.graph.bricks, &k.pos);
    }
    layout
}

/// The memoised evaluation and per-brick scores of a level; both follow accept/unaccept.
fn memos(m: &Rc<Model>, params: Signal<Params>) -> (ReadSignal<Eval>, ReadSignal<Vec<BrickScore>>) {
    let accepted = |m: &Model| -> Vec<bool> { m.cards.iter().map(|c| c.accepted.get()).collect() };
    let eval = {
        let m = m.clone();
        create_memo(move || params.with(|p| evaluate(&m.g, &m.diags, &accepted(&m), p)))
    };
    let scores = {
        let m = m.clone();
        create_memo(move || brick_scores(&m.g, &m.diags, &accepted(&m)))
    };
    (eval, scores)
}

impl Cx {
    /// The state of level `lv`: shared signals from `sh`, fresh pan/zoom/drag and layout.
    /// `stage` is the stage size in pixels when known (the caller measures it).
    pub fn new(sh: &Shared, lv: usize, stage: Option<(f64, f64)>) -> Cx {
        let params = sh.params;
        let levels = sh.live.levels.get_clone_untracked();
        let design = sh.live.design.get_clone_untracked();
        let level = &levels[lv];
        // After a patch, boxes that were on screen stay where they were.
        let keep = sh
            .live
            .keep
            .get_clone_untracked()
            .filter(|k| k.level == level.path);
        let editing = params.with_untracked(|p| p.editing());
        let layout = level_layout(level, design.as_deref(), editing, keep.as_ref());
        let planned = planned_pairs(level);
        let plan0 = level.graph.edges.len();
        let m = Rc::new(Model {
            cache: sh.cache.clone(),
            g: with_frame_edges(&level.graph),
            levels,
            lv,
            layout,
            diags: sh.diags.clone(),
            cards: sh.cards.clone(),
            fns: sh.fns.clone(),
            planned,
            plan0,
            design,
        });
        let stage = create_signal(stage);
        let view = create_signal(match (&keep, stage.get()) {
            (Some(k), _) => k.view,
            (None, Some(s)) => fit_view(&m.layout, s, fit_room(editing)),
            (None, None) => Vw {
                x: 0.0,
                y: 0.0,
                k: 1.0,
            },
        });
        let (eval, scores) = memos(&m, params);
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
            fit: keep.is_none(),
            live: sh.live,
            ed: sh.ed,
            nav: sh.nav,
            bus: sh.bus,
        }
    }

    /// Change the params. The page mirrors them into the URL (see `components::app`).
    pub fn go(&self, p: Params) {
        self.params.set(p);
    }

    /// The editor is on (edit mode, the design loaded).
    pub fn editing(&self) -> bool {
        self.params.with(|p| editing(self.m.design.as_deref(), p))
    }

    /// The config file of the current level, relative to the root config's directory.
    pub fn level_file(&self) -> Option<String> {
        let ch = self.m.design.as_deref()?;
        ch.files.get(&self.level().path).cloned()
    }

    /// The view that fits the level into a stage of size `s`.
    pub fn fit(&self, s: (f64, f64)) -> Vw {
        fit_view(&self.m.layout, s, fit_room(self.editing()))
    }

    /// The current level.
    pub fn level(&self) -> &LevelView {
        &self.m.levels[self.m.lv]
    }

    /// The routed path of planned pair `a -> b` for box positions `p` (`None` when the pair is
    /// not routed).
    pub fn planned_route(&self, a: usize, b: usize, p: &[Rect]) -> Option<layout::Route> {
        let i = self.m.planned.iter().position(|&x| x == (a, b))?;
        self.m.layout.path(self.m.plan0 + i, p)
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
