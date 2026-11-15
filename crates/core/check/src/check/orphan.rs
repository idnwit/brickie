//! `review/orphan`: an accept whose id matches no current finding.

use crate::config::Level;
use crate::diag::{Finding, RuleRef, Span};
use brickie_model::codes::REVIEW_ORPHAN;

pub(super) struct Orphan {
    id: String,
    file: String,
    line: u32,
}

impl Orphan {
    /// `text` is the review file, to find the line of the entry (0 when not found).
    pub(super) fn new(id: &str, file: &str, text: &str) -> Self {
        let quoted = format!("\"{id}\"");
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with("id") && l.contains(&quoted));
        // The line of the `[[accept]]` header above the `id`, else of the `id` itself.
        let line = at.map_or(0, |i| {
            let head = lines[..i].iter().rposition(|l| l.trim() == "[[accept]]");
            head.unwrap_or(i) as u32 + 1
        });
        Orphan {
            id: id.to_string(),
            file: file.to_string(),
            line,
        }
    }
}

impl Finding for Orphan {
    fn code(&self) -> String {
        REVIEW_ORPHAN.into()
    }
    fn level(&self) -> Level {
        Level::Warn
    }
    fn subject(&self) -> String {
        self.id.clone()
    }
    fn message(&self) -> String {
        format!("accepted finding `{}` no longer exists", self.id)
    }
    fn spans(&self) -> Vec<Span> {
        vec![Span {
            file: self.file.clone(),
            line: self.line,
            col: None,
            len: None,
            label: None,
        }]
    }
    fn rule(&self) -> Option<RuleRef> {
        None
    }
    fn help(&self) -> Option<String> {
        Some("the code moved (the finding has a new id: review it again) or it was fixed: remove the entry".into())
    }
}
