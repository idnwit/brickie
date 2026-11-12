//! The design editor's state: its signals ([`Edit`], shared by every level) and the pure
//! helpers the components use to build ops, place bricks, hit-test drops, draw typed links and
//! patch the levels in place. No DOM access.

mod hit;
mod link;
mod ops;
mod patch;
mod sheet;
mod words;

pub use hit::*;
pub use link::*;
pub use ops::*;
pub use patch::*;
pub use sheet::*;
pub use words::*;

use brickie_web_api::ChangeView;
use brickie_web_api::change::{BrickKind, LinkCheck, LinkOption, Plan};
use sycamore::prelude::*;

/// A brick being added: its kind and canvas (world) position; the name is asked inline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draft {
    pub kind: BrickKind,
    pub x: f64,
    pub y: f64,
}

/// The modal dialog of the change bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Dialog {
    #[default]
    None,
    Open,
    /// The publish plan (`None` while it loads).
    Publish(Option<Plan>),
    /// Published: offer to measure now (the cache predates the new bricks).
    Published,
    Discard,
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
    pub x: f64,
    pub y: f64,
}

/// A rule selected on the canvas or in the Rules tab: its physical file and index there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRef {
    pub file: String,
    pub index: usize,
}

/// Where a dragged brick would go on release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    /// Into the sibling at this index (held long enough).
    Into(usize),
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
    /// A brick tile dragged from the toolbar, and the pointer (client coordinates).
    pub carry: Signal<Option<(BrickKind, f64, f64)>>,
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
    /// A brick held over a sibling: the sibling and since when (ms).
    pub hold: Signal<Option<(usize, f64)>>,
    pub target: Signal<Option<MoveTarget>>,
    pub rule: Signal<Option<RuleRef>>,
    /// Link options of the selected brick (the inspector's "now" lines).
    pub links: Signal<Vec<LinkOption>>,
    pub dialog: Signal<Dialog>,
    /// Design panel tab: `inspect`, `rules`, `level`, `files` or `ops`.
    pub tab: Signal<&'static str>,
    /// Ops tab filter: `all`, `agent`, `human`, `proposed` or `conflict`.
    pub ops_filter: Signal<&'static str>,
    /// The delete confirmation popover is open.
    pub confirm: Signal<bool>,
    pub toolbar_open: Signal<bool>,
    /// Diff view: the semantic diff over the preview.
    pub diff: Signal<bool>,
    /// The last error of a change request (empty: none).
    pub msg: Signal<String>,
    /// The last op done, shown briefly.
    pub toast: Signal<String>,
    pub busy: Signal<bool>,
}

impl Edit {
    pub(super) fn new() -> Edit {
        Edit {
            tool: create_signal(Tool::Select),
            brick_tool: create_signal(None),
            carry: create_signal(None),
            draft: create_signal(None),
            rename: create_signal(None),
            link_type: create_signal(None),
            scope: create_signal("all"),
            connect: create_signal(None),
            check: create_signal(None),
            hot: create_signal(None),
            hold: create_signal(None),
            target: create_signal(None),
            rule: create_signal(None),
            links: create_signal(Vec::new()),
            dialog: create_signal(Dialog::None),
            tab: create_signal("inspect"),
            ops_filter: create_signal("all"),
            confirm: create_signal(false),
            toolbar_open: create_signal(true),
            diff: create_signal(false),
            msg: create_signal(String::new()),
            toast: create_signal(String::new()),
            busy: create_signal(false),
        }
    }

    /// Drop the armed tool, a drag in progress and any open form; false when nothing was.
    pub fn cancel(&self) -> bool {
        let armed = self.brick_tool.get().is_some() || self.link_type.get().is_some();
        let open = self.draft.get().is_some()
            || self.rename.with(Option::is_some)
            || self.connect.with(Option::is_some)
            || self.carry.get().is_some()
            || self.confirm.get();
        self.brick_tool.set(None);
        self.link_type.set(None);
        self.draft.set(None);
        self.rename.set(None);
        self.connect.set(None);
        self.carry.set(None);
        self.confirm.set(false);
        armed || open
    }
}

/// The editor is on: the preview is shown and a change is open.
pub(super) fn editing(change: Option<&ChangeView>, p: &brickie_web_api::Params) -> bool {
    change.is_some() && p.editing()
}
