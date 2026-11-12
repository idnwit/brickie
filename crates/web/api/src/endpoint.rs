//! The HTTP JSON endpoints. One marker type per endpoint implements [`Endpoint`]; the server
//! mounts it at `E::PATH` with handlers typed by [`ReqOf`] and [`ResOf`], and the UI calls the
//! same `E::PATH` and decodes the same `E::Res`.
//!
//! `Req` is the input: the JSON body of a POST endpoint, or the query string of a GET endpoint
//! (the viewer [`Params`]). `()` means the endpoint takes no input.

use crate::{
    DiagView, EdgeItemView, EdgeUsesRequest, FnRow, InterfaceRequest, InterfaceView, LevelView,
    Params, ReviewReply, ReviewRequest,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait Endpoint {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned;
    type Res: Serialize + DeserializeOwned;
}

/// The request type of endpoint `E`.
pub type ReqOf<E> = <E as Endpoint>::Req;
/// The response type of endpoint `E`.
pub type ResOf<E> = <E as Endpoint>::Res;

/// `POST /review`: accept (`reason`) or unaccept (`reason: null`) a finding.
pub struct Review;
impl Endpoint for Review {
    const PATH: &'static str = "/review";
    type Req = ReviewRequest;
    type Res = ReviewReply;
}

/// `GET /api/graph`: every level of the brick tree.
pub struct Graph;
impl Endpoint for Graph {
    const PATH: &'static str = "/api/graph";
    type Req = ();
    type Res = Vec<LevelView>;
}

/// `GET /api/diagnostics?<params>`: the findings visible under `params`.
pub struct Diagnostics;
impl Endpoint for Diagnostics {
    const PATH: &'static str = "/api/diagnostics";
    type Req = Params;
    type Res = Vec<DiagView>;
}

/// `GET /api/functions?<params>`: the function rows matching `params`.
pub struct Functions;
impl Endpoint for Functions {
    const PATH: &'static str = "/api/functions";
    type Req = Params;
    type Res = Vec<FnRow>;
}

/// `POST /api/interface`: the declared interface of a brick, with uses.
pub struct Interface;
impl Endpoint for Interface {
    const PATH: &'static str = "/api/interface";
    type Req = InterfaceRequest;
    type Res = InterfaceView;
}

/// `POST /api/edge-uses`: every reference site from one brick into another, by used item.
pub struct EdgeUses;
impl Endpoint for EdgeUses {
    const PATH: &'static str = "/api/edge-uses";
    type Req = EdgeUsesRequest;
    type Res = Vec<EdgeItemView>;
}

/// `GET /`: the page (HTML, query = [`Params`]).
pub const PAGE_PATH: &str = "/";
/// `GET /code/?path=&line=&col=`: the code viewer (`codeview`), embedded by the source popup
/// and the target of every source link (it needs JavaScript).
pub const CODE_PATH: &str = "/code/";
