//! State that outlives a level: the URL params, the review state of every finding, the popup
//! and menus, and the page data. [`Cx`] is rebuilt from it each time the level changes.

use super::{Cx, Edit};
use brickie_web_api::*;
use std::cell::RefCell;
use std::rc::Rc;
use sycamore::prelude::*;

/// One diagnostic with its client-side review state.
#[derive(Clone)]
pub struct Card {
    pub d: DiagView,
    pub accepted: Signal<bool>,
    /// The reason of the accepted state.
    pub reason: Signal<String>,
    /// The reason input box.
    pub input: Signal<String>,
    pub note: Signal<String>,
    pub busy: Signal<bool>,
}

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
    pub levels: Rc<Vec<LevelView>>,
    pub diags: Vec<DiagView>,
    pub cards: Rc<Vec<Card>>,
    pub fns: Vec<FnRow>,
    /// The open design change.
    pub change: Option<Rc<ChangeView>>,
    pub ed: Edit,
}

/// The [`Cx`] of the level on screen, for window-level handlers registered once.
pub type Cur = Rc<RefCell<Option<Cx>>>;

impl Shared {
    pub fn new(data: PageData, initial: Params) -> Shared {
        let cards = data
            .diags
            .iter()
            .cloned()
            .map(|d| Card {
                accepted: create_signal(d.accepted),
                reason: create_signal(d.accepted_reason.clone().unwrap_or_default()),
                input: create_signal(String::new()),
                note: create_signal(String::new()),
                busy: create_signal(false),
                d,
            })
            .collect();
        Shared {
            params: create_signal(initial),
            hover: create_signal(None),
            modal: create_signal(None),
            menu: create_signal(String::new()),
            q: create_signal(String::new()),
            cache: data.cache,
            levels: Rc::new(data.levels),
            diags: data.diags,
            cards: Rc::new(cards),
            fns: data.functions,
            change: data.change.map(Rc::new),
            ed: Edit::new(),
        }
    }

    /// Index of the level the params name (the root when unknown).
    pub fn level_of(&self, p: &Params) -> usize {
        level_index(&self.levels, p.level_path())
    }
}

/// The current [`Cx`], cloned out of the cell (the borrow ends before any signal changes).
pub fn current(cur: &Cur) -> Option<Cx> {
    cur.borrow().clone()
}
