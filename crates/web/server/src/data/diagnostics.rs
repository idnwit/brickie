//! Diagnostics: the check's `Diagnostic` converted field by field, with the source line of the
//! primary span cut around it.

use brickie_core::config::Level;
use brickie_core::diag::{Diagnostic, Span};
use brickie_ui::*;
use std::collections::HashMap;
use std::path::Path;

/// Source lines of cache files, read once per request.
pub struct Lines<'a> {
    root: &'a Path,
    files: HashMap<String, Vec<String>>,
}

impl<'a> Lines<'a> {
    pub fn new(root: &'a Path) -> Self {
        Lines {
            root,
            files: HashMap::new(),
        }
    }
    fn line(&mut self, file: &str, line: u32) -> Option<&str> {
        let root = self.root;
        let lines = self.files.entry(file.to_string()).or_insert_with(|| {
            std::fs::read_to_string(root.join(file))
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default()
        });
        lines
            .get((line as usize).checked_sub(1)?)
            .map(String::as_str)
    }
}

fn span_view(s: &Span) -> SpanView {
    SpanView {
        file: s.file.clone(),
        line: s.line,
        col: s.col,
        len: s.len,
        label: s.label.clone(),
    }
}

pub fn diag_view(d: &Diagnostic, lines: &mut Lines) -> DiagView {
    let snippet = d.span.as_ref().filter(|s| s.line > 0).and_then(|s| {
        let text = lines.line(&s.file, s.line)?;
        let (before, hit, after) = match s.col {
            Some(c) => split_bytes(text, c as usize - 1, s.len.unwrap_or(1) as usize),
            None => (String::new(), String::new(), text.to_string()),
        };
        Some(Snippet {
            line: s.line,
            before,
            hit,
            after,
        })
    });
    DiagView {
        id: d.id.clone(),
        level: match d.level {
            Level::Error => "error",
            Level::Warn => "warn",
        }
        .into(),
        code: d.code.clone(),
        message: d.message.clone(),
        span: d.span.as_ref().map(span_view),
        spans: d.spans.iter().map(span_view).collect(),
        rule: d.rule.as_ref().map(|r| RuleView {
            name: r.name.clone(),
            loc: r.loc.clone(),
        }),
        help: d.help.clone(),
        notes: d
            .notes
            .iter()
            .map(|n| NoteView {
                text: n.text.clone(),
                span: n.span.as_ref().map(span_view),
            })
            .collect(),
        bricks: d.bricks.clone(),
        brick_level: d.brick_level.clone(),
        accepted: d.accepted,
        accepted_reason: d.accepted_reason.clone(),
        snippet,
    }
}

/// Split `text` at byte offset `start` for `len` bytes, clamped to char boundaries.
pub fn split_bytes(text: &str, start: usize, len: usize) -> (String, String, String) {
    let mut a = start.min(text.len());
    while !text.is_char_boundary(a) {
        a -= 1;
    }
    let mut b = (a + len).min(text.len());
    while !text.is_char_boundary(b) {
        b += 1;
    }
    (
        text[..a].to_string(),
        text[a..b].to_string(),
        text[b..].to_string(),
    )
}
