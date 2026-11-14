//! The page data a write can change without a reload: the levels and the design. An
//! `EditReply` patches them in place; [`Keep`] carries the canvas over the re-render.

use super::{Cx, Keep, patch_levels};
use brickie_web_api::{DesignState, LevelView};
use std::cell::RefCell;
use std::rc::Rc;
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub struct Live {
    pub levels: Signal<Rc<Vec<LevelView>>>,
    pub design: Signal<Option<Rc<DesignState>>>,
    /// Bumped after a patch: the page re-renders the level on screen.
    pub rev: Signal<u64>,
    /// The canvas state the next render restores (taken once).
    pub keep: Signal<Option<Keep>>,
}

impl Live {
    pub(super) fn new(levels: Vec<LevelView>, design: Option<DesignState>) -> Live {
        Live {
            levels: create_signal(Rc::new(levels)),
            design: create_signal(design.map(Rc::new)),
            rev: create_signal(0),
            keep: create_signal(None),
        }
    }

    /// Replace the design and patch `levels` in, keeping the canvas as `keep` says (`None`:
    /// fit the view).
    pub fn apply(&self, design: DesignState, levels: Vec<LevelView>, keep: Option<Keep>) {
        let now = self
            .levels
            .with_untracked(|l| patch_levels(l, levels, &design.files));
        self.levels.set_silent(Rc::new(now));
        self.design.set_silent(Some(Rc::new(design)));
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
