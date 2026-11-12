//! Findings as marks: every span of a diagnostic in the file, with a link selecting the finding
//! in the brickie viewer. Accepted findings are info marks. Also the open error and warning
//! counts per file, for the explorer badges.

use brickie_check::check::Loaded;
use brickie_check::config::Level as DLevel;
use brickie_check::diag::{Diagnostic, Span as DSpan};
use brickie_web_api::Params;
use codeview_port::{Badge, Level, Mark, Pos, Span};
use std::collections::HashMap;

/// The brickie viewer URL selecting finding `d` at its level.
pub fn link(d: &Diagnostic) -> String {
    let lv = d.brick_level.clone().filter(|l| l != "root");
    Params {
        item: d.id.clone(),
        lv: lv.unwrap_or_default(),
        ..Params::default()
    }
    .url()
}

fn span(s: &DSpan) -> Span {
    let col = s.col.unwrap_or(1);
    let end = match (s.col, s.len) {
        (Some(c), Some(n)) => c + n.max(1),
        (Some(c), None) => c + 1,
        // No column: the whole line (0 = to the end).
        _ => 0,
    };
    Span {
        start: Pos { line: s.line, col },
        end: Pos {
            line: s.line,
            col: end,
        },
    }
}

/// The marks of `path`.
pub fn marks(l: &Loaded, path: &str) -> Vec<Mark> {
    let mut out = Vec::new();
    for d in &l.diagnostics {
        let level = match (d.accepted, d.level) {
            (true, _) => Level::Info,
            (false, DLevel::Error) => Level::Error,
            (false, DLevel::Warn) => Level::Warn,
        };
        let spans = d.span.iter().chain(&d.spans);
        for s in spans.filter(|s| s.file == path && s.line > 0) {
            let mut message = format!("{}: {}", d.code, d.message);
            if let Some(label) = &s.label {
                message += &format!(" ({label})");
            }
            if d.accepted {
                message += " [accepted]";
            }
            out.push(Mark {
                span: span(s),
                level,
                message,
                link: Some(link(d)),
            });
        }
    }
    out.sort_by_key(|m| (m.span.start.line, m.span.start.col));
    out.dedup();
    out
}

/// Open (not accepted) errors and warnings by the file of their primary span.
pub fn counts(l: &Loaded) -> HashMap<&str, Badge> {
    let mut out: HashMap<&str, Badge> = HashMap::new();
    for d in l.diagnostics.iter().filter(|d| !d.accepted) {
        let Some(s) = &d.span else { continue };
        let b = out.entry(s.file.as_str()).or_default();
        match d.level {
            DLevel::Error => b.errors += 1,
            DLevel::Warn => b.warns += 1,
        }
    }
    out
}
