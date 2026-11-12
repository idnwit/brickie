//! The embedded bundle (see `build.rs`): `index.html`, the app and Monaco under `static/`.

use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

mod files {
    include!(concat!(env!("OUT_DIR"), "/bundle.rs"));
}

/// Whether the viewer bundle (page and app wasm) is embedded.
pub fn embedded() -> bool {
    find("index.html").is_some() && find("codeview_app_bg.wasm").is_some()
}

fn find(name: &str) -> Option<&'static [u8]> {
    files::FILES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
}

const MISSING: &str = "<!doctype html><meta charset=utf-8><title>code viewer</title>\
<body style=\"font:14px system-ui;padding:2em\"><h1>The code viewer is not built</h1>\
<p>This binary was built without the viewer bundle. Run <code>scripts/build-codeview.sh</code> \
(or <code>scripts/build-ui.sh</code>, which builds every bundle) and rebuild.</p></body>";

pub async fn index() -> Response {
    match find("index.html").filter(|_| embedded()) {
        Some(b) => (
            [(header::CACHE_CONTROL, "no-cache")],
            Html(String::from_utf8_lossy(b).into_owned()),
        )
            .into_response(),
        None => (StatusCode::OK, Html(MISSING)).into_response(),
    }
}

fn mime(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "ttf" => "font/ttf",
        "html" => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub async fn file(Path(name): Path<String>) -> Response {
    match find(&name).filter(|_| name != "index.html") {
        Some(b) => (
            [
                (header::CONTENT_TYPE, mime(&name)),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            b,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
