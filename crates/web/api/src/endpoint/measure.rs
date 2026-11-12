//! The server's measure job (`/api/measure`): one job at a time (single flight). Starting it
//! writes the cache and passes the server's write guard.

use super::Endpoint;
use crate::MeasureJob;

/// `GET /api/measure`: the measure job's state.
pub struct MeasureStatus;
impl Endpoint for MeasureStatus {
    const PATH: &'static str = "/api/measure";
    type Req = ();
    type Res = MeasureJob;
}

/// `POST /api/measure`: start the measure job, or return the one already running.
pub struct MeasureStart;
impl Endpoint for MeasureStart {
    const PATH: &'static str = "/api/measure";
    type Req = ();
    type Res = MeasureJob;
}
