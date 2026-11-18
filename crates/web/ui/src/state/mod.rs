//! Reactive viewer state: the selection, focus, layers and evaluation derived from the URL, the
//! graph and the diagnostics, plus the signals the components share ([`Cx`]).
//!
//! The state is a function of [`Params`](brickie_web_api::Params) (the URL) and the page data:
//! [`Sel`] says what is selected, [`Lay`] which data layers are on, [`evaluate`] what to draw
//! and how, and [`Nav`] the view history. It holds no DOM and fetches nothing (the components
//! and the `platform` wrappers do), so most of it is plain, testable functions.
//!
//! # Examples
//!
//! The URL is the state: changing a selection or a preset yields new params.
//!
//! ```
//! use brickie_ui::state::{Lay, Nav, Sel, evaluate, with_preset};
//! use brickie_web_api::{BrickView, GraphView, Params};
//!
//! let graph = GraphView {
//!     rows: vec!["core".into()],
//!     bricks: vec![BrickView { name: "api".into(), ..BrickView::default() }],
//!     edges: vec![],
//! };
//!
//! // A brick in the URL becomes a selection; an unknown name is ignored.
//! let p = Params::from_query("?brick=api");
//! assert_eq!(Sel::from_params(&p, &graph), Sel::Brick(0));
//! assert_eq!(Sel::from_params(&Params::from_query("brick=nope"), &graph), Sel::None);
//!
//! // Presets and layers.
//! let viol = with_preset(&p, "viol");
//! assert_eq!(viol.preset(), "viol");
//! let lay = Lay::from_params(&Params::from_query("lay=%2Btests"));
//! assert!(lay.on("tests") && lay.on("edges"));
//!
//! // What the page shows for these params.
//! let ev = evaluate(&graph, &[], &p);
//! assert_eq!(ev.node, vec![0]);
//!
//! // Back and forward over pushed URLs.
//! let mut nav = Nav::new("/".into());
//! nav.push("/?lv=root/web".into());
//! assert_eq!(nav.back(), Some("/"));
//! ```
//!
//! # Key types and functions
//!
//! - [`Cx`], [`Shared`]: the signals of one level and of the whole page ([`Cx`] is rebuilt when
//!   the level changes; [`Shared`] outlives it).
//! - [`Sel`]: nothing, one brick or one link, kept in the URL.
//! - [`Lay`], [`with_preset`], [`with_toggle`]: data layers and presets.
//! - [`evaluate`], [`list_findings`]: which findings pass the layers, how each edge is drawn,
//!   per-brick finding counts and the focus.
//! - [`Nav`]: the Back and Forward history; [`on_event`]: what to do with a server event.
//!
//! # Invariants
//!
//! The URL holds every view choice, so [`Sel::apply`] and [`with_preset`] return new
//! [`Params`](brickie_web_api::Params) rather than mutate signals; the components then navigate.
//! Ids that no longer exist (a renamed brick) are ignored instead of failing.

mod ctx;
mod editor;
mod eval;
mod events;
mod focus;
mod history;
mod lanes;
mod layers;
mod links;
mod live;
mod selection;
mod shared;
mod stage;

pub use ctx::*;
pub use editor::*;
pub use eval::*;
pub use events::*;
pub use focus::*;
pub use history::*;
pub use lanes::*;
pub use layers::*;
pub use links::*;
pub use live::*;
pub use selection::*;
pub use shared::*;
pub use stage::*;
