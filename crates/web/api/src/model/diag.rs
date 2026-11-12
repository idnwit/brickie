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
    /// Seconds since the cache file was written, when the page was built (`None`: unknown).
    #[serde(default)]
    pub age_secs: Option<u64>,
    /// Files changed since the measure (`stale/file` findings).
    #[serde(default)]
    pub stale_files: usize,
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

    /// The code viewer at this span.
    pub fn href(&self) -> String {
        code_href(&self.file, self.line, self.col)
    }
}

/// The code viewer at a file, line and byte column (`/code/?path=&line=&col=`); line 0 opens
/// the file without moving.
pub fn code_href(file: &str, line: u32, col: Option<u32>) -> String {
    let mut s = format!("{}?path={}&line={line}", crate::CODE_PATH, encode(file));
    if let Some(c) = col {
        s += &format!("&col={c}");
    }
    s
}

/// The code viewer with the Bricks explorer scoped to the brick `node` (its level path, for
/// example `root/web/server`); the viewer opens the brick's entry file.
pub fn code_brick_href(node: &str) -> String {
    format!("{}?filter={}&view=Bricks", crate::CODE_PATH, encode(node))
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
    /// The level whose children `bricks` names (`root`, `root/web`); none for non-brick findings.
    #[serde(default)]
    pub brick_level: Option<String>,
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

/// Group and sub-layer key of a diagnostic code.
pub fn group_sub(code: &str) -> (&'static str, String) {
    if let Some(r) = code.strip_prefix("deps/") {
        let sub = match r {
            "deny" => "d-deny",
            "default" => "d-default",
            "private" => "d-private",
            "cycle" => "d-cycle",
            _ => return ("deps", "deps".into()),
        };
        return ("deps", sub.into());
    }
    if let Some(m) = code.strip_prefix("complexity/") {
        return ("cx", format!("c-{m}"));
    }
    if code.starts_with("interface/") {
        return ("iface", "i-unused".into());
    }
    let sub = match code {
        "stale/file" => "s-file",
        "bricks/unmapped" => "s-unmapped",
        _ => "s-other",
    };
    ("stale", sub.into())
}

/// Whether a diagnostic passes the filters. `accepted` is passed separately because the
/// client changes it locally after a review.
pub fn diag_visible(d: &DiagView, accepted: bool, p: &Params) -> bool {
    let code_ok = code_matches(&d.code, &p.code);
    (p.level.is_empty() || d.level == p.level)
        && code_ok
        && (p.grp.is_empty() || group_sub(&d.code).0 == p.grp)
        && (p.brick.is_empty() || p.bricks().iter().any(|b| d.bricks.iter().any(|x| x == b)))
        && match p.status.as_str() {
            "open" => !accepted,
            "accepted" => accepted,
            _ => true,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_links_open_the_code_viewer() {
        assert_eq!(
            code_href("crates/a b.rs", 12, Some(5)),
            "/code/?path=crates/a%20b.rs&line=12&col=5"
        );
        assert_eq!(code_href("a.rs", 3, None), "/code/?path=a.rs&line=3");
    }
}
