//! State that outlives a level: the URL params, the popup
//! and menus, and the page data. [`Cx`] is rebuilt from it each time the level changes.

use super::{Bus, Cx, Edit, Live, Nav};
use brickie_web_api::*;
use std::cell::RefCell;
use std::rc::Rc;
use sycamore::prelude::*;

#[derive(Clone)]
pub struct Shared {
    pub params: Signal<Params>,
    /// The finding (card index) hovered in the left list.
    pub hover: Signal<Option<usize>>,
    pub modal: Signal<Option<String>>,
    /// Id of the open dropdown menu.
    pub menu: Signal<String>,
    /// Text filter of the findings list.
    pub q: Signal<String>,
    pub cache: CacheInfo,
    pub diags: Vec<DiagView>,
    pub fns: Vec<FnRow>,
    /// The levels and the design (patched in place by the editor).
    pub live: Live,
    pub ed: Edit,
    /// Back / Forward history of the view.
    pub nav: Signal<Nav>,
    /// What the server's event stream drives (pill, measure job).
    pub bus: Bus,
}

/// The [`Cx`] of the level on screen, for window-level handlers registered once.
pub type Cur = Rc<RefCell<Option<Cx>>>;

impl Shared {
    pub fn new(data: PageData, initial: Params) -> Shared {
        Shared {
            nav: create_signal(Nav::new(initial.url())),
            bus: Bus::new(),
            params: create_signal(initial),
            hover: create_signal(None),
            modal: create_signal(None),
            menu: create_signal(String::new()),
            q: create_signal(String::new()),
            cache: data.cache,
            diags: data.diags,
            fns: data.functions,
            live: Live::new(data.levels, data.design),
            ed: Edit::new(),
        }
    }

    /// Index of the level the params name (the root when unknown).
    pub fn level_of(&self, p: &Params) -> usize {
        self.live.levels.with(|l| level_index(l, p.level_path()))
    }
}

/// The current [`Cx`], cloned out of the cell (the borrow ends before any signal changes).
pub fn current(cur: &Cur) -> Option<Cx> {
    cur.borrow().clone()
}
