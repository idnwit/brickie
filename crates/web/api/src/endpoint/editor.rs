//! The design editor's endpoints (`/api/design/*`). Writes (`edit`, `move`, `undo`, `layout`,
//! `relayout`) pass the server's write guard (loopback bind, same-origin request). A write based
//! on another version than the head answers 409 with the `design/stale` text (header
//! `x-brickie-head`: the head version); the page reloads the design and retries.

use super::Endpoint;
use crate::design::{LinkCheck, LinkOption};
use crate::{
    DesignState, EditReply, EditRequest, LayoutRequest, LevelView, LevelsRequest, LinkCheckRequest,
    LinksRequest, MoveRequest, RelayoutRequest, UndoRequest,
};

/// `GET /api/design`: the design at its head.
pub struct DesignGet;
impl Endpoint for DesignGet {
    const PATH: &'static str = "/api/design";
    type Req = ();
    type Res = DesignState;
}

/// `POST /api/design/edit`: apply ops as one human version.
pub struct DesignEdit;
impl Endpoint for DesignEdit {
    const PATH: &'static str = "/api/design/edit";
    type Req = EditRequest;
    type Res = EditReply;
}

/// `POST /api/design/move`: `move_brick` built from brick paths, as one human version.
pub struct DesignMove;
impl Endpoint for DesignMove {
    const PATH: &'static str = "/api/design/move";
    type Req = MoveRequest;
    type Res = EditReply;
}

/// `POST /api/design/undo`: revert a version as a new human version.
pub struct DesignUndo;
impl Endpoint for DesignUndo {
    const PATH: &'static str = "/api/design/undo";
    type Req = UndoRequest;
    type Res = EditReply;
}

/// `POST /api/design/link-check`: whether a rule would change what its level decides (read
/// only); the editor refuses a redundant drop.
pub struct DesignLinkCheck;
impl Endpoint for DesignLinkCheck {
    const PATH: &'static str = "/api/design/link-check";
    type Req = LinkCheckRequest;
    type Res = LinkCheck;
}

/// `POST /api/design/links`: the links a brick may draw to its siblings (read only).
pub struct DesignLinks;
impl Endpoint for DesignLinks {
    const PATH: &'static str = "/api/design/links";
    type Req = LinksRequest;
    type Res = Vec<LinkOption>;
}

/// `POST /api/design/levels`: levels by path, marked as in edit mode (read only).
pub struct DesignLevels;
impl Endpoint for DesignLevels {
    const PATH: &'static str = "/api/design/levels";
    type Req = LevelsRequest;
    type Res = Vec<LevelView>;
}

/// `POST /api/design/layout`: set or clear a brick's canvas position.
pub struct DesignLayout;
impl Endpoint for DesignLayout {
    const PATH: &'static str = "/api/design/layout";
    type Req = LayoutRequest;
    type Res = ();
}

/// `POST /api/design/relayout`: clear every canvas position of a level (auto layout).
pub struct DesignRelayout;
impl Endpoint for DesignRelayout {
    const PATH: &'static str = "/api/design/relayout";
    type Req = RelayoutRequest;
    type Res = ();
}
