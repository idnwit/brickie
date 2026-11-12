//! `GET /source`: one cached file with line numbers, the requested line or span highlighted.
//!
//! Only files listed in the cache can be read, and the resolved path must stay inside the
//! cached workspace root and not be a symlink.

use crate::{AppState, Failure, render};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct SourceQuery {
    file: String,
    line: u32,
    col: Option<u32>,
    end_col: Option<u32>,
}

/// Largest file the popup renders.
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Read `file` (a cache key) from `root`; 400 for path syntax that could traverse, 404 for
/// anything not listed in the cache or not a regular file inside the root.
pub fn read_confined(
    root: &Path,
    listed: impl Fn(&str) -> bool,
    file: &str,
) -> Result<String, Failure> {
    let fail = |c: StatusCode, m: &str| Failure(c, m.to_string());
    if file.is_empty()
        || file.starts_with('/')
        || file.contains('\0')
        || file.contains('\\')
        || file
            .split('/')
            .any(|c| c == ".." || c == "." || c.is_empty())
    {
        return Err(fail(StatusCode::BAD_REQUEST, "invalid file path"));
    }
    if !listed(file) {
        return Err(fail(StatusCode::NOT_FOUND, "file is not in the cache"));
    }
    let path = root.join(file);
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|_| fail(StatusCode::NOT_FOUND, "no such file"))?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(fail(
            StatusCode::NOT_FOUND,
            "not a regular file of reasonable size",
        ));
    }
    // Symlinked directories along the way: the canonical path must stay under the root.
    let real = path
        .canonicalize()
        .map_err(|_| fail(StatusCode::NOT_FOUND, "no such file"))?;
    let root = root
        .canonicalize()
        .map_err(|_| fail(StatusCode::NOT_FOUND, "no workspace"))?;
    if !real.starts_with(&root) {
        return Err(fail(StatusCode::NOT_FOUND, "file is outside the workspace"));
    }
    let bytes = std::fs::read(&real).map_err(|_| fail(StatusCode::NOT_FOUND, "unreadable file"))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub async fn source(
    State(st): State<AppState>,
    Query(q): Query<SourceQuery>,
) -> Result<Html<String>, Failure> {
    let loaded = st.load()?;
    let cache = &loaded.cache;
    let text = read_confined(
        Path::new(&cache.workspace_root),
        |f| cache.files.contains_key(f),
        &q.file,
    )?;
    Ok(Html(render::source_html(
        &q.file, &text, q.line, q.col, q.end_col,
    )))
}
