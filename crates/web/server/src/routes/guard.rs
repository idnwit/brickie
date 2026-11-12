//! The write guard of every endpoint that writes files (`/review`, `/api/change/*`, `POST /api/measure`): the
//! server must be bound to loopback, and the request must come from the page it served (the
//! `Host` names that loopback address, an `Origin` matches it). This stops other web pages
//! (CSRF, DNS rebinding) and other machines from editing the workspace.

use crate::{AppState, Failure};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use std::net::IpAddr;

/// Extracting it passes the guard.
pub struct Guard;

impl FromRequestParts<AppState> for Guard {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Guard, Failure> {
        check(&st.bind.host, st.bind.port, &parts.headers)
            .map(|()| Guard)
            .map_err(|m| Failure(StatusCode::FORBIDDEN, m))
    }
}

fn loopback(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    h.eq_ignore_ascii_case("localhost") || h.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// `host[:port]` (IPv6 in brackets) split into host and port.
fn split(authority: &str) -> (&str, Option<u16>) {
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !h.ends_with(':') && (!h.contains(':') || h.ends_with(']')) => {
            (h, p.parse().ok())
        }
        _ => (authority, None),
    };
    (host, port)
}

fn header<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

/// `Ok` when a write may proceed; the error says why not.
pub fn check(bind: &str, port: Option<u16>, h: &HeaderMap) -> Result<(), String> {
    if !loopback(bind) {
        return Err(format!(
            "editing is disabled: the server is bound to {bind}, not to loopback (use --bind 127.0.0.1)"
        ));
    }
    let host = header(h, "host").ok_or("missing Host header")?;
    let (name, p) = split(host);
    if !loopback(name) || (port.is_some() && p != port) {
        return Err(format!("Host {host} is not this server"));
    }
    if let Some(origin) = header(h, "origin") {
        if origin != format!("http://{host}") {
            return Err(format!("cross-origin request from {origin}"));
        }
    } else if header(h, "sec-fetch-site").is_some_and(|s| s != "same-origin" && s != "none") {
        return Err("cross-site request".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, v.parse().unwrap());
        }
        h
    }

    #[test]
    fn guard_needs_loopback_and_same_origin() {
        let ok = headers(&[
            ("host", "127.0.0.1:8080"),
            ("origin", "http://127.0.0.1:8080"),
        ]);
        assert!(check("127.0.0.1", Some(8080), &ok).is_ok());
        assert!(check("::1", Some(8080), &ok).is_ok());
        assert!(check("0.0.0.0", Some(8080), &ok).is_err());
        assert!(check("127.0.0.1", Some(9000), &ok).is_err());
        let evil = headers(&[("host", "127.0.0.1:8080"), ("origin", "http://evil.test")]);
        assert!(check("127.0.0.1", Some(8080), &evil).is_err());
        let rebind = headers(&[("host", "evil.test:8080")]);
        assert!(check("127.0.0.1", Some(8080), &rebind).is_err());
        assert!(check("127.0.0.1", None, &HeaderMap::new()).is_err());
        let cli = headers(&[("host", "localhost:8080")]);
        assert!(check("localhost", Some(8080), &cli).is_ok());
        let site = headers(&[("host", "[::1]:8080"), ("sec-fetch-site", "cross-site")]);
        assert!(check("::1", Some(8080), &site).is_err());
        assert_eq!(split("[::1]:80"), ("[::1]", Some(80)));
    }
}
