//! The design editor's state: its signals ([`Edit`], shared by every level) and the pure
//! helpers the components use to build ops, place bricks, hit-test drops, draw typed links and
//! patch the levels in place. No DOM access.

mod dnd;
mod hit;
mod link;
mod ops;
mod patch;
mod refs;
mod retry;
mod sheet;
mod undo;
mod words;

pub use dnd::*;
pub use hit::*;
pub use link::*;
pub use ops::*;
pub use patch::*;
pub use refs::*;
pub use retry::*;
pub use sheet::*;
pub use undo::*;
pub use words::*;

use brickie_web_api::DesignState;
use brickie_web_api::design::{BrickKind, LinkCheck, LinkOption, RuleKey, RuleSpec};
use sycamore::prelude::*;

/// A brick being added: its kind, canvas (world) position and where it goes; the name (and
/// for a new lane, the tag) is asked inline.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub kind: BrickKind,
    pub x: f64,
    pub y: f64,
    pub dest: Dest,
}

/// Where a new brick (or a lane drop) goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dest {
    /// On this level, with these tags.
    Here(Vec<String>),
    /// Inside the brick of this name (a child of its level).
    Into(String),
    /// On this level, in a new tag lane (asked inline).
    NewLane,
}

/// What follows the pointer outside a brick drag: a new tile from the toolbar, or a brick
/// lifted off its level (spring-loaded zoom) that keeps following across levels.
#[derive(Debug, Clone, PartialEq)]
pub enum Carry {
    Tile(BrickKind),
    /// Brick `name` of level `level` (`root/core`).
    Brick {
        level: String,
        name: String,
    },
}

/// A drag hovering a group brick: it opens after a short hold (spring-loaded zoom).
#[derive(Debug, Clone, PartialEq)]
pub struct Spring {
    /// Which hover this is: a timer started for an earlier one does nothing.
    pub id: u32,
    pub level: String,
    pub name: String,
    /// The pointer (client coordinates).
    pub x: f64,
    pub y: f64,
}

/// A brick waiting to be moved: cut (`Ctrl+X`) at `level`, pasted (`Ctrl+V`) elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub level: String,
    pub name: String,
}

/// Pointer tool of the floating toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Hand,
}

/// A typed port being dragged: the rule `from -> ?` it will add, the pointer in world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Connect {
    pub from: String,
    pub ty: LinkType,
    /// Dragging the arrowhead of this rule: the drop changes its `to` (`set_rule`).
    pub retarget: Option<RuleRef>,
    pub x: f64,
    pub y: f64,
}

/// A rule selected on the canvas or in the Rules tab: its physical file and its identity
/// there (never its position, which another writer may change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRef {
    pub file: String,
    pub key: RuleKey,
}

impl RuleRef {
    /// The reference of rule `r` written in `file`.
    pub fn of(file: &str, r: &RuleSpec) -> RuleRef {
        RuleRef {
            file: file.to_string(),
            key: rule_key(r),
        }
    }
}

/// The identity of a planned edge's rule.
pub fn unused_key(e: &brickie_web_api::UnusedLink) -> RuleKey {
    let mut kinds = e.kinds.clone();
    kinds.sort();
    RuleKey {
        from: e.from.clone(),
        to: e.to.clone(),
        kinds: Some(kinds),
        scope: Some(e.scope.clone()),
    }
}

/// The identity of a rule: from, to, kinds (as a set) and scope (`prod` when absent).
pub fn rule_key(r: &RuleSpec) -> RuleKey {
    let mut kinds = r.kinds.clone();
    kinds.sort();
    RuleKey {
        from: r.from.clone(),
        to: r.to.clone(),
        kinds: Some(kinds),
        scope: Some(r.scope.clone().unwrap_or_else(|| "prod".into())),
    }
}

/// Where a dragged brick would go on release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    /// Into the sibling at this index (like a folder).
    Into(usize),
    /// Into a tag lane (`None`: the untagged lane).
    Lane(Option<String>),
    /// Onto the "+ new tag lane" zone (the tag is asked).
    NewLane,
    /// Up to the level at this path (frame edge or breadcrumb).
    Up(String),
    /// Not allowed here, with the reason.
    No(String),
}

/// The editor's signals (all `Copy`).
#[derive(Clone, Copy)]
pub struct Edit {
    pub tool: Signal<Tool>,
    /// The brick tile armed for the next click on the canvas.
    pub brick_tool: Signal<Option<BrickKind>>,
    /// A tile or lifted brick following the pointer (client coordinates).
    pub carry: Signal<Option<(Carry, f64, f64)>>,
    /// The group brick a drag hovers, armed to open (spring-loaded zoom).
    pub spring: Signal<Option<Spring>>,
    /// The brick cut with `Ctrl+X`.
    pub clip: Signal<Option<Clip>>,
    /// The keyboard shortcuts sheet (`?`) is open.
    pub help: Signal<bool>,
    pub draft: Signal<Option<Draft>>,
    /// The brick renamed inline on its box.
    pub rename: Signal<Option<String>>,
    /// The armed link type: ports show on every brick.
    pub link_type: Signal<Option<LinkType>>,
    /// Scope of new rules: `prod`, `test` or `all`.
    pub scope: Signal<&'static str>,
    pub connect: Signal<Option<Connect>>,
    /// The link-check answer for the target under the pointer while connecting.
    pub check: Signal<Option<(String, LinkCheck)>>,
    /// The brick under the pointer (ports show on it).
    pub hot: Signal<Option<usize>>,
    /// The brick under the pointer while dragging (the drop target's box).
    pub over: Signal<Option<usize>>,
    /// The ghost's result label while dragging (empty: not dragging).
    pub aim_label: Signal<String>,
    /// A brick dropped on the new-lane zone: its name and where the tag box shows (world).
    pub lane_ask: Signal<Option<(String, f64, f64)>>,
    pub target: Signal<Option<MoveTarget>>,
    pub rule: Signal<Option<RuleRef>>,
    /// Link options of the selected brick (the inspector's "now" lines).
    pub links: Signal<Vec<LinkOption>>,
    /// Design panel tab: `inspect`, `rules`, `level`, `files` or `history`.
    pub tab: Signal<&'static str>,
    /// The delete confirmation popover is open.
    pub confirm: Signal<bool>,
    pub toolbar_open: Signal<bool>,
    /// The last error of a design request (empty: none).
    pub msg: Signal<String>,
    /// The last write done, or what another writer changed, shown briefly.
    pub toast: Signal<String>,
    /// The version the toast offers to undo (another writer's version).
    pub toast_undo: Signal<Option<u64>>,
    pub busy: Signal<bool>,
}

impl Edit {
    pub(super) fn new() -> Edit {
        Edit {
            tool: create_signal(Tool::Select),
            brick_tool: create_signal(None),
            carry: create_signal(None),
            spring: create_signal(None),
            clip: create_signal(None),
            help: create_signal(false),
            draft: create_signal(None),
            rename: create_signal(None),
            link_type: create_signal(None),
            scope: create_signal("all"),
            connect: create_signal(None),
            check: create_signal(None),
            hot: create_signal(None),
            over: create_signal(None),
            aim_label: create_signal(String::new()),
            lane_ask: create_signal(None),
            target: create_signal(None),
            rule: create_signal(None),
            links: create_signal(Vec::new()),
            tab: create_signal("inspect"),
            confirm: create_signal(false),
            toolbar_open: create_signal(true),
            msg: create_signal(String::new()),
            toast: create_signal(String::new()),
            toast_undo: create_signal(None),
            busy: create_signal(false),
        }
    }

    /// Drop the armed tool, a drag in progress and any open form; false when nothing was.
    pub fn cancel(&self) -> bool {
        let armed = self.brick_tool.get().is_some() || self.link_type.get().is_some();
        let open = self.draft.with(Option::is_some)
            || self.lane_ask.with(Option::is_some)
            || self.rename.with(Option::is_some)
            || self.connect.with(Option::is_some)
            || self.carry.with(Option::is_some)
            || self.clip.with(Option::is_some)
            || self.help.get()
            || self.confirm.get();
        self.brick_tool.set(None);
        self.link_type.set(None);
        self.draft.set(None);
        self.lane_ask.set(None);
        self.aim_label.set(String::new());
        self.target.set(None);
        self.over.set(None);
        self.rename.set(None);
        self.connect.set(None);
        self.carry.set(None);
        self.spring.set(None);
        self.clip.set(None);
        self.help.set(false);
        self.confirm.set(false);
        armed || open
    }
}

/// The editor is on: edit mode, and the design loaded.
pub(super) fn editing(design: Option<&DesignState>, p: &brickie_web_api::Params) -> bool {
    design.is_some() && p.editing()
}

/// A small design for tests: two levels, a shared include, rules and exports.
#[cfg(test)]
pub(crate) use sheet::tests::change as tests_design;
