//! One line per finding, for coding agents: `error[deps/deny] 3fa9c1d2 src/a.rs:42:17: message`.

use super::text::{relative, summary_line};
use super::{Diagnostic, Span};
use std::path::Path;

fn place(s: &Span, root: &Path) -> String {
    let file = relative(&s.file, root);
    match (s.line, s.col) {
        (0, _) => file,
        (l, None) => format!("{file}:{l}"),
        (l, Some(c)) => format!("{file}:{l}:{c}"),
    }
}

/// The line of one finding. Notes without a location follow the
/// message in parentheses.
fn line(d: &Diagnostic, root: &Path) -> String {
    let at = d
        .span
        .as_ref()
        .map_or(String::new(), |s| format!(" {}", place(s, root)));
    let notes: String = (d.notes.iter())
        .filter(|n| n.span.is_none())
        .map(|n| format!(" ({})", n.text))
        .collect();
    format!("error[{}] {}{at}: {}{notes}\n", d.code, d.id, d.message)
}

/// Every finding on one line, then the summary line. Paths under `root` print
/// relative to it.
pub fn render_short(ds: &[Diagnostic], root: &Path) -> String {
    let mut out: String = ds.iter().map(|d| line(d, root)).collect();
    out += &summary_line(ds, false);
    out
}
