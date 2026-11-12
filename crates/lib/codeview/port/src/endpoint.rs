//! The HTTP endpoints the viewer calls. One marker type per endpoint implements [`Endpoint`];
//! every endpoint is a `POST` of `Req` as JSON answered with `Res` as JSON. `PATH` is relative
//! to where the host mounts the viewer (`api/file` under `/code/` is `/code/api/file`).

use crate::{Band, FileText, Location, Mark, Pos, View};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub trait Endpoint {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned;
    type Res: Serialize + DeserializeOwned;
}

/// A file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathReq {
    pub path: String,
}

/// A position in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtReq {
    pub path: String,
    pub pos: Pos,
}

/// The explorer trees.
pub struct Views;
impl Endpoint for Views {
    const PATH: &'static str = "api/views";
    type Req = ();
    type Res = Vec<View>;
}

/// A file's text; 404 when the host has no such file, 400 for an invalid path.
pub struct File;
impl Endpoint for File {
    const PATH: &'static str = "api/file";
    type Req = PathReq;
    type Res = FileText;
}

/// The marks of a file.
pub struct Marks;
impl Endpoint for Marks {
    const PATH: &'static str = "api/marks";
    type Req = PathReq;
    type Res = Vec<Mark>;
}

/// The bands of a file (every band of the whole file).
pub struct Bands;
impl Endpoint for Bands {
    const PATH: &'static str = "api/bands";
    type Req = PathReq;
    type Res = Vec<Band>;
}

/// Where the symbol at a position is defined.
pub struct Definition;
impl Endpoint for Definition {
    const PATH: &'static str = "api/definition";
    type Req = AtReq;
    type Res = Vec<Location>;
}

/// Where the symbol at a position is used.
pub struct References;
impl Endpoint for References {
    const PATH: &'static str = "api/references";
    type Req = AtReq;
    type Res = Vec<Location>;
}

/// A relative path with only normal components (no root, `.`, `..`, empty parts, `\` or NUL):
/// the only paths the host router passes to a provider.
pub fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
}
