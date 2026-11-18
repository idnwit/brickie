//! The structured diagnostic every finding renders to (JSON output is its serialisation).

use serde::Serialize;

/// A source location. `line == 0` means the whole file. `col` and `len` are 1-based UTF-8
/// byte offsets on `line`; `col` is `None` when unknown.
#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub file: String,
    pub line: u32,
    pub col: Option<u32>,
    pub len: Option<u32>,
    pub label: Option<String>,
}

/// An extra remark attached to a [`Diagnostic`], optionally pointing at a location.
#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub text: String,
    pub span: Option<Span>,
}

/// The configured rule that fired; `loc` is `file:line` of its table in the config.
#[derive(Debug, Clone, Serialize)]
pub struct RuleRef {
    pub name: String,
    pub loc: Option<String>,
}

/// A brick-to-brick edge a finding is about (one hop of a cycle), with the evidence location.
#[derive(Debug, Clone, Serialize)]
pub struct DiagEdge {
    pub from: String,
    pub to: String,
    pub span: Option<Span>,
}

/// One finding, as printed by `brickie check` and serialised to JSON.
///
/// `code` is one of the stable strings in [`crate::codes`]; `id` identifies the finding within
/// one run (the viewer uses it to select a finding).
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub id: String,
    /// A diagnostic code, for example `deps/unlinked` or `complexity/cognitive`.
    pub code: String,
    pub message: String,
    /// The main location; `None` when the finding is about no single place.
    pub span: Option<Span>,
    /// Further locations of the same finding (more references).
    pub spans: Vec<Span>,
    pub rule: Option<RuleRef>,
    /// How to fix the finding, in plain words.
    pub help: Option<String>,
    pub notes: Vec<Note>,
    /// Bricks the finding is about (both ends of an edge, the members of a cycle, the brick
    /// of a function); used by the webview to filter. Empty when it concerns no brick.
    pub bricks: Vec<String>,
    /// Level of the brick tree the finding is reported at (`root`, `root/web`); the names in
    /// `bricks` and `edges` are children of that level. `None` for findings about no brick.
    pub brick_level: Option<String>,
    /// Brick edges the finding is about (the hops of a cycle path); the webview draws them as
    /// violating. Empty for findings that are not about brick edges.
    pub edges: Vec<DiagEdge>,
}
