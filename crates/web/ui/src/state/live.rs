//! The page data an op can change without a reload: the levels and the open change. A
//! `ChangeReply` patches them in place; [`Keep`] carries the canvas over the re-render.

use super::{Cx, Keep, patch_levels};
use brickie_web_api::{ChangeView, LevelView};
use std::cell::RefCell;
use std::rc::Rc;
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct Live {
    pub levels: Signal<Rc<Vec<LevelView>>>,
    pub change: Signal<Option<Rc<ChangeView>>>,
    /// Bumped after a patch: the page re-renders the level on screen.
    pub rev: Signal<u64>,
    /// The canvas state the next render restores (taken once).
    pub keep: Signal<Option<Keep>>,
}

impl Live {
    pub(super) fn new(levels: Vec<LevelView>, change: Option<ChangeView>) -> Live {
        Live {
            levels: create_signal(Rc::new(levels)),
            change: create_signal(change.map(Rc::new)),
            rev: create_signal(0),
            keep: create_signal(None),
        }
    }

    /// Replace the change and patch `levels` in, keeping the canvas as `keep` says.
    pub fn apply(&self, change: ChangeView, levels: Vec<LevelView>, keep: Option<Keep>) {
        let now = self
            .levels
            .with_untracked(|l| patch_levels(l, levels, &change.files));
        self.levels.set_silent(Rc::new(now));
        self.change.set_silent(Some(Rc::new(change)));
        self.keep.set_silent(keep);
        self.rev.update(|r| *r += 1);
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Cx>> = const { RefCell::new(None) };
}

/// Record the [`Cx`] of the level on screen (the page does after each render).
pub fn set_current_cx(cx: &Cx) {
    CURRENT.with(|c| *c.borrow_mut() = Some(cx.clone()));
}

/// The [`Cx`] of the level on screen.
pub fn current_cx() -> Option<Cx> {
    CURRENT.with(|c| c.borrow().clone())
}
