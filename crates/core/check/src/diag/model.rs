//! How a finding becomes a [`Diagnostic`] (the value types live in `brickie-model`).

use brickie_facts::sha256_hex;
pub use brickie_model::{DiagEdge, Diagnostic, Level, Note, RuleRef, Span};

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
    fn brick_level(&self) -> Option<String> {
        None
    }
    /// Full level paths (`root/web/api`) of the bricks that own the finding, for scoped checks.
    /// Default: every brick in `bricks` at `brick_level`; none without a level.
    fn owners(&self) -> Vec<String> {
        let level = self.brick_level();
        let level = level.as_deref();
        (self.bricks().iter())
            .filter_map(|b| level.map(|l| format!("{l}/{b}")))
            .collect()
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
        brick_level: f.brick_level(),
        edges: f.edges(),
        accepted: false,
        accepted_reason: None,
    }
}
