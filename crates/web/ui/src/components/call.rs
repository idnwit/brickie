//! Typed calls to the server's JSON endpoints: the path and both payload types come from the
//! [`Endpoint`] contract in `brickie-web-api`; the browser wrapper only moves JSON text.

use crate::platform;
use brickie_web_api::{Endpoint, ReqOf, ResOf};

/// POST `req` to `E::PATH` and hand the decoded reply (or an error message) to `done`.
pub fn call<E: Endpoint>(req: &ReqOf<E>, done: impl FnOnce(Result<ResOf<E>, String>) + 'static) {
    let body = match serde_json::to_string(req) {
        Ok(b) => b,
        Err(e) => return done(Err(e.to_string())),
    };
    platform::post_json(E::PATH, body, move |res| {
        done(res.and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string())))
    });
}
