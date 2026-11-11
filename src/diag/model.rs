//! The structured diagnostic every finding renders to (JSON output is its serialisation).

use crate::cache::sha256_hex;
use crate::config::Level;
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

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub id: String,
    pub level: Level,
    pub code: String,
    pub message: String,
    pub span: Option<Span>,
    /// Further locations of the same finding (more references).
    pub spans: Vec<Span>,
    pub rule: Option<RuleRef>,
    pub help: Option<String>,
    pub notes: Vec<Note>,
    /// Bricks the finding is about (both ends of an edge, the members of a cycle, the brick
    /// of a function); used by the webview to filter. Empty when it concerns no brick.
    pub bricks: Vec<String>,
    /// Brick edges the finding is about (the hops of a cycle path); the webview draws them as
    /// violating. Empty for findings that are not about brick edges.
    pub edges: Vec<DiagEdge>,
    pub accepted: bool,
    pub accepted_reason: Option<String>,
}

/// One rule type's finding. Implementations only describe the parts; `diagnostic` assembles.
pub trait Finding {
    fn code(&self) -> String;
    fn level(&self) -> Level;
    /// Stable identity within the code; the id is a hash of code and subject.
    fn subject(&self) -> String;
    fn message(&self) -> String;
    /// First is the primary location.
    fn spans(&self) -> Vec<Span>;
    fn rule(&self) -> Option<RuleRef>;
    fn help(&self) -> Option<String>;
    fn notes(&self) -> Vec<Note> {
        Vec::new()
    }
    fn bricks(&self) -> Vec<String> {
        Vec::new()
    }
    fn edges(&self) -> Vec<DiagEdge> {
        Vec::new()
    }
}

pub fn finding_id(code: &str, subject: &str) -> String {
    let h = sha256_hex(format!("{code}\0{subject}").as_bytes());
    h[..8].to_string()
}

pub fn diagnostic(f: &dyn Finding) -> Diagnostic {
    let code = f.code();
    let mut spans = f.spans().into_iter();
    Diagnostic {
        id: finding_id(&code, &f.subject()),
        level: f.level(),
        code,
        message: f.message(),
        span: spans.next(),
        spans: spans.collect(),
        rule: f.rule(),
        help: f.help(),
        notes: f.notes(),
        bricks: f.bricks(),
        edges: f.edges(),
        accepted: false,
        accepted_reason: None,
    }
}
