//! The design editor's endpoints (`/api/change/*`). Every POST but `links`, `link-check` and
//! `levels` writes and passes the server's write guard (loopback bind, same-origin request).
//! Writes that change the design answer a [`ChangeReply`] so the editor patches in place.

use super::Endpoint;
use crate::change::{LinkCheck, LinkOption, OpRecord, Plan, ReviewEntry};
use crate::{
    ChangeLayoutRequest, ChangeLevelsRequest, ChangeLinkCheckRequest, ChangeLinksRequest,
    ChangeMoveRequest, ChangeOpRequest, ChangeOpenRequest, ChangeReapplyRequest, ChangeReply,
    ChangeRetractRequest, ChangeReviewRequest, ChangeView, LevelView,
};

/// `GET /api/change`: the open change (`null`: none is open).
pub struct ChangeState;
impl Endpoint for ChangeState {
    const PATH: &'static str = "/api/change";
    type Req = ();
    type Res = Option<ChangeView>;
}

/// `POST /api/change/open`: open a change with its intent.
pub struct ChangeOpen;
impl Endpoint for ChangeOpen {
    const PATH: &'static str = "/api/change/open";
    type Req = ChangeOpenRequest;
    type Res = ChangeView;
}

/// `POST /api/change/op`: record a human op.
pub struct ChangeOp;
impl Endpoint for ChangeOp {
    const PATH: &'static str = "/api/change/op";
    type Req = ChangeOpRequest;
    type Res = ChangeReply<OpRecord>;
}

/// `POST /api/change/move`: record `move_brick` built from brick paths.
pub struct ChangeMove;
impl Endpoint for ChangeMove {
    const PATH: &'static str = "/api/change/move";
    type Req = ChangeMoveRequest;
    type Res = ChangeReply<OpRecord>;
}

/// `POST /api/change/review`: accept or reject an op.
pub struct ChangeReview;
impl Endpoint for ChangeReview {
    const PATH: &'static str = "/api/change/review";
    type Req = ChangeReviewRequest;
    type Res = ChangeReply<ReviewEntry>;
}

/// `POST /api/change/retract`: take back a human op (undo); the log keeps it, rejected.
pub struct ChangeRetract;
impl Endpoint for ChangeRetract {
    const PATH: &'static str = "/api/change/retract";
    type Req = ChangeRetractRequest;
    type Res = ChangeReply<ReviewEntry>;
}

/// `POST /api/change/reapply`: re-record a conflicting op as a human op against the current
/// design and reject the original; `done` is the new op.
pub struct ChangeReapply;
impl Endpoint for ChangeReapply {
    const PATH: &'static str = "/api/change/reapply";
    type Req = ChangeReapplyRequest;
    type Res = ChangeReply<OpRecord>;
}

/// `POST /api/change/link-check`: whether a rule would change what its level decides (read
/// only); the editor refuses a redundant drop.
pub struct ChangeLinkCheck;
impl Endpoint for ChangeLinkCheck {
    const PATH: &'static str = "/api/change/link-check";
    type Req = ChangeLinkCheckRequest;
    type Res = LinkCheck;
}

/// `POST /api/change/levels`: preview levels by path, marked like `/api/graph?change=1` (read
/// only).
pub struct ChangeLevels;
impl Endpoint for ChangeLevels {
    const PATH: &'static str = "/api/change/levels";
    type Req = ChangeLevelsRequest;
    type Res = Vec<LevelView>;
}

/// `POST /api/change/links`: the links a brick may draw to its siblings (read only).
pub struct ChangeLinks;
impl Endpoint for ChangeLinks {
    const PATH: &'static str = "/api/change/links";
    type Req = ChangeLinksRequest;
    type Res = Vec<LinkOption>;
}

/// `POST /api/change/layout`: set or clear a brick's canvas position.
pub struct ChangeLayout;
impl Endpoint for ChangeLayout {
    const PATH: &'static str = "/api/change/layout";
    type Req = ChangeLayoutRequest;
    type Res = ();
}

/// `GET /api/change/plan`: what publish would write, and why it would refuse.
pub struct ChangePublishPlan;
impl Endpoint for ChangePublishPlan {
    const PATH: &'static str = "/api/change/plan";
    type Req = ();
    type Res = Plan;
}

/// `POST /api/change/publish`: write the config files, scaffold, close the change.
pub struct ChangePublish;
impl Endpoint for ChangePublish {
    const PATH: &'static str = "/api/change/publish";
    type Req = ();
    type Res = Plan;
}

/// `POST /api/change/discard`: delete the change (the config files are untouched).
pub struct ChangeDiscard;
impl Endpoint for ChangeDiscard {
    const PATH: &'static str = "/api/change/discard";
    type Req = ();
    type Res = ();
}
