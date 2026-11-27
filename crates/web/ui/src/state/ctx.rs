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
    pub fns: Vec<FnRow>,
    /// The unused rule pairs routed by the layout, as layout edges
    /// `plan0 + i` (empty outside edit mode).
    pub unused: Vec<(usize, usize)>,
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
}

/// The state of the level on screen: params, hover, pan and zoom, drag, popup and the memoised
/// evaluation. Cheap to clone (signals are handles); rebuilt by [`Cx::new`] whenever the level
/// changes, from the longer-lived [`Shared`].
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
    /// Per brick score (findings and complexity excess).
    pub scores: ReadSignal<Vec<BrickScore>>,
    /// Entry transition of the canvas: `zin`, `zout` or none.
    pub anim: &'static str,
    /// Fit the view once the stage is measured (false when a patch kept the view).
    pub fit: bool,
    pub live: Live,
    pub ed: Edit,
    pub nav: Signal<Nav>,
    pub bus: Bus,
    /// Which side panels are shown.
    pub panes: Signal<Panes>,
}

/// Extra vertical breathing room between the level frame and the outside `inbound`/`outbound`
/// ghost rows (leaves room for the frame header and clean orthogonal boundary channels).
const BOUNDARY_GAP: f64 = 64.0;
/// Horizontal spacing when centring an unanchored ghost row over the level frame.
const GHOST_GAP: f64 = 20.0;

struct BoundaryGroups {
    inner: Vec<usize>,
    inbound: Vec<usize>,
    outbound: Vec<usize>,
}

fn partition_boundary(g: &GraphView, len: usize) -> BoundaryGroups {
    let mut inner = Vec::new();
    let mut inbound = Vec::new();
    let mut outbound = Vec::new();
    for (i, b) in g.bricks.iter().take(len).enumerate() {
        if !b.ghost {
            inner.push(i);
        } else if b.tag == "inbound" {
            inbound.push(i);
        } else {
            outbound.push(i);
        }
    }
    BoundaryGroups {
        inner,
        inbound,
        outbound,
    }
}

fn shift_inbound_gap(layout: &mut Layout, frame_y: f64, bg: &BoundaryGroups) -> f64 {
    if bg.inbound.is_empty() {
        return 0.0;
    }
    let in_bot = bg
        .inbound
        .iter()
        .map(|&i| layout.pos[i].y + layout.pos[i].h)
        .fold(f64::NEG_INFINITY, f64::max);
    let shift = in_bot + BOUNDARY_GAP - frame_y;
    if shift <= 0.0 {
        return 0.0;
    }
    for &i in bg.inner.iter().chain(&bg.outbound) {
        layout.pos[i].y += shift;
    }
    for r in &mut layout.rows {
        if !r.ids.iter().all(|id| bg.inbound.contains(id)) {
            r.y += shift;
        }
    }
    for d in layout.dpos.iter_mut().flatten() {
        d.1 += shift;
    }
    shift
}

fn shift_outbound_gap(layout: &mut Layout, frame_bot: f64, outbound: &[usize]) {
    if outbound.is_empty() {
        return;
    }
    let out_top = outbound
        .iter()
        .map(|&i| layout.pos[i].y)
        .fold(f64::INFINITY, f64::min);
    let dy = frame_bot + BOUNDARY_GAP - out_top;
    if dy <= 0.0 {
        return;
    }
    for &i in outbound {
        layout.pos[i].y += dy;
    }
    for r in &mut layout.rows {
        if !r.ids.is_empty() && r.ids.iter().all(|id| outbound.contains(id)) {
            r.y += dy;
        }
    }
}

fn shift_boundary_vertical(layout: &mut Layout, frame: &Rect, bg: &BoundaryGroups) {
    let shift_inner = shift_inbound_gap(layout, frame.y, bg);
    shift_outbound_gap(layout, frame.y + frame.h + shift_inner, &bg.outbound);
}

fn row_width(pos: &[Rect], ids: &[usize]) -> f64 {
    ids.iter().map(|&i| pos[i].w).sum::<f64>() + GHOST_GAP * ids.len().saturating_sub(1) as f64
}

fn has_direct_edge(g: &GraphView, extra: &[(usize, usize)], ids: &[usize], to_frame: bool) -> bool {
    g.edges
        .iter()
        .filter(|e| !(to_frame && is_boundary(g, e)))
        .any(|e| ids.contains(&e.from) || ids.contains(&e.to))
        || extra
            .iter()
            .any(|&(a, b)| ids.contains(&a) || ids.contains(&b))
}

fn center_unanchored_ghosts(
    layout: &mut Layout,
    g: &GraphView,
    extra: &[(usize, usize)],
    frame: &Rect,
    bg: &BoundaryGroups,
    to_frame: bool,
) {
    let groups = [&bg.inbound, &bg.outbound];
    let max_ghost_w = groups
        .into_iter()
        .filter(|ids| !ids.is_empty() && !has_direct_edge(g, extra, ids, to_frame))
        .map(|ids| row_width(&layout.pos, ids))
        .fold(0.0_f64, f64::max);
    let mut frame_cx = frame.x + frame.w / 2.0;
    let min_cx = layout.px0 + max_ghost_w / 2.0;
    if min_cx > frame_cx {
        let dx = min_cx - frame_cx;
        for &i in &bg.inner {
            layout.pos[i].x += dx;
        }
        for d in layout.dpos.iter_mut().flatten() {
            d.0 += dx;
        }
        frame_cx = min_cx;
    }
    for group in groups {
        if group.is_empty() || has_direct_edge(g, extra, group, to_frame) {
            continue;
        }
        let mut sorted = group.clone();
        sorted.sort_by(|&a, &b| layout.pos[a].x.total_cmp(&layout.pos[b].x));
        let mut x = (frame_cx - row_width(&layout.pos, &sorted) / 2.0).max(layout.px0);
        for &i in &sorted {
            layout.pos[i].x = x;
            x += layout.pos[i].w + GHOST_GAP;
        }
    }
}

/// Give `inbound` / `outbound` ghost rows comfortable vertical clearance outside the level frame
/// and centre ghost rows that have no direct child-to-ghost edges over the frame's middle.
fn space_boundary_rows(
    layout: &mut Layout,
    g: &GraphView,
    extra: &[(usize, usize)],
    to_frame: bool,
) {
    let bg = partition_boundary(g, layout.pos.len());
    if bg.inner.is_empty() || (bg.inbound.is_empty() && bg.outbound.is_empty()) {
        return;
    }
    let Some(frame) = frame_of(&g.bricks, &layout.pos, None) else {
        return;
    };
    shift_boundary_vertical(layout, &frame, &bg);
    center_unanchored_ghosts(layout, g, extra, &frame, &bg, to_frame);
    let max_x = layout.pos.iter().map(|r| r.x + r.w).fold(0.0_f64, f64::max);
    let max_y = layout.pos.iter().map(|r| r.y + r.h).fold(0.0_f64, f64::max);
    layout.w = max_x + 20.0;
    layout.h = max_y + 50.0;
    layout.reroute();
}

/// The layout of `level`: automatic from its active edge set (omitting boundary child edges when
/// `to_frame` is true; with planned rule pairs after them), then the editor's stored positions,
/// then the positions a patch keeps on screen.
fn level_layout(
    level: &LevelView,
    design: Option<&DesignState>,
    editing: bool,
    to_frame: bool,
    keep: Option<&Keep>,
) -> Layout {
    let extra = unused_pairs(level, editing);
    let mut layout = layout::layout(&layout_input(&level.graph, &extra, to_frame));
    space_boundary_rows(&mut layout, &level.graph, &extra, to_frame);
    let before = layout.pos.clone();
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
    if layout.pos != before {
        layout.reroute();
    }
    layout
}

/// The memoised evaluation and per-brick scores of a level; the eval follows the params.
fn memos(m: &Rc<Model>, params: Signal<Params>) -> (ReadSignal<Eval>, ReadSignal<Vec<BrickScore>>) {
    let eval = {
        let m = m.clone();
        create_memo(move || params.with(|p| evaluate(&m.g, &m.diags, p)))
    };
    let scores = {
        let m = m.clone();
        create_memo(move || brick_scores(&m.g, &m.diags))
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
        let (editing, to_frame) = params.with_untracked(|p| (p.editing(), p.to_frame()));
        let layout = level_layout(level, design.as_deref(), editing, to_frame, keep.as_ref());
        let unused = unused_pairs(level, editing);
        let plan0 = level.graph.edges.len();
        let m = Rc::new(Model {
            cache: sh.cache.clone(),
            g: with_frame_edges(&level.graph),
            levels,
            lv,
            layout,
            diags: sh.diags.clone(),
            fns: sh.fns.clone(),
            unused,
            plan0,
            design,
        });
        let stage = create_signal(stage);
        let view = create_signal(match (&keep, stage.get()) {
            (Some(k), _) => k.view,
            (None, Some(s)) => fit_view(&m.layout, s, fit_room(editing)),
            (None, None) => fit_view(&m.layout, (920.0, 780.0), fit_room(editing)),
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
            panes: sh.panes,
        }
    }

    /// Change the params. The page mirrors them into the URL (see `app`).
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
    pub fn unused_route(&self, a: usize, b: usize, p: &[Rect]) -> Option<layout::Route> {
        let i = self.m.unused.iter().position(|&x| x == (a, b))?;
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
