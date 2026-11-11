//! Diagnostics as the webview receives them (the CLI's `Diagnostic` field for field), the cache
//! header and the review request/reply.

use super::{Params, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheInfo {
    pub workspace: String,
    pub commit: Option<String>,
    pub dirty: bool,
    /// UTC time the cache file was written.
    pub measured: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanView {
    pub file: String,
    pub line: u32,
    pub col: Option<u32>,
    pub len: Option<u32>,
    pub label: Option<String>,
}

impl SpanView {
    /// `file:line:col`, `file:line` or just `file` (line 0 = whole file).
    pub fn loc(&self) -> String {
        match (self.line, self.col) {
            (0, _) => self.file.clone(),
            (l, None) => format!("{}:{l}", self.file),
            (l, Some(c)) => format!("{}:{l}:{c}", self.file),
        }
    }

    /// The source popup page for this span.
    pub fn href(&self) -> String {
        source_href(&self.file, self.line, self.col, self.len)
    }
}

pub fn source_href(file: &str, line: u32, col: Option<u32>, len: Option<u32>) -> String {
    let mut s = format!("/source?file={}&line={line}", encode(file));
    if let Some(c) = col {
        s += &format!("&col={c}&end_col={}", c + len.unwrap_or(1));
    }
    if line > 0 {
        s += &format!("#L{line}");
    }
    s
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteView {
    pub text: String,
    pub span: Option<SpanView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleView {
    pub name: String,
    /// `file:line` in the config.
    pub loc: Option<String>,
}

/// One source line with the primary span cut out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub line: u32,
    pub before: String,
    pub hit: String,
    pub after: String,
}

/// The same fields as the CLI's `Diagnostic` (plus a snippet the CLI prints from disk).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagView {
    pub id: String,
    /// `error` | `warn`.
    pub level: String,
    pub code: String,
    pub message: String,
    pub span: Option<SpanView>,
    pub spans: Vec<SpanView>,
    pub rule: Option<RuleView>,
    pub help: Option<String>,
    pub notes: Vec<NoteView>,
    pub bricks: Vec<String>,
    pub accepted: bool,
    pub accepted_reason: Option<String>,
    pub snippet: Option<Snippet>,
}

/// Reply of `POST /review`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewReply {
    pub id: String,
    pub accepted: bool,
    pub reason: Option<String>,
}

/// Body of `POST /review`: `reason: None` unaccepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub id: String,
    pub reason: Option<String>,
}

/// A diagnostic code filter: empty, exact (`deps/deny`) or a family prefix (`complexity`).
pub fn code_matches(code: &str, filter: &str) -> bool {
    filter.is_empty()
        || code == filter
        || code
            .strip_prefix(filter)
            .is_some_and(|r| r.starts_with('/'))
}

/// Whether a diagnostic passes the filters. `accepted` is passed separately because the
/// client changes it locally after a review.
pub fn diag_visible(d: &DiagView, accepted: bool, p: &Params) -> bool {
    let code_ok = code_matches(&d.code, &p.code);
    (p.level.is_empty() || d.level == p.level)
        && code_ok
        && (p.grp.is_empty() || crate::state::group_sub(&d.code).0 == p.grp)
        && (p.brick.is_empty() || p.bricks().iter().any(|b| d.bricks.iter().any(|x| x == b)))
        && match p.status.as_str() {
            "open" => !accepted,
            "accepted" => accepted,
            _ => true,
        }
}
