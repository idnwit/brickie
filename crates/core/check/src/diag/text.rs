//! Cargo-style text printer: one consumer of the [`Diagnostic`] value.

use super::{Diagnostic, Span, summarize};
use brickie_model::Level;
use std::io::IsTerminal;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl Color {
    pub fn enabled(self) -> bool {
        match self {
            Color::Always => true,
            Color::Never => false,
            Color::Auto => {
                std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
                    && std::io::stdout().is_terminal()
            }
        }
    }
}

/// How many extra locations / notes are shown before the rest is summarised.
const SHOWN: usize = 3;

struct Paint(bool);

impl Paint {
    fn wrap(&self, code: &str, s: &str) -> String {
        if self.0 {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
    fn level(&self, l: Level, s: &str) -> String {
        self.wrap(
            match l {
                Level::Error => "1;31",
                Level::Warn => "1;33",
            },
            s,
        )
    }
    fn blue(&self, s: &str) -> String {
        self.wrap("1;34", s)
    }
}

fn loc(s: &Span) -> String {
    match (s.line, s.col) {
        (0, _) => s.file.clone(),
        (l, None) => format!("{}:{l}", s.file),
        (l, Some(c)) => format!("{}:{l}:{c}", s.file),
    }
}

fn word(l: Level) -> &'static str {
    match l {
        Level::Error => "error",
        Level::Warn => "warning",
    }
}

/// The source line of a span with its gutter and, when the column is known, a caret line.
struct Snippet<'a> {
    text: String,
    span: &'a Span,
    level: Level,
}

impl<'a> Snippet<'a> {
    /// `None` for a whole-file span or a line that cannot be read from `root`.
    fn load(span: &'a Span, level: Level, root: &Path) -> Option<Self> {
        if span.line == 0 {
            return None;
        }
        let text = std::fs::read_to_string(root.join(&span.file)).ok()?;
        let text = text.lines().nth(span.line as usize - 1)?.to_string();
        Some(Snippet { text, span, level })
    }

    /// (chars before the caret, caret count) for the 1-based byte `col` and `len`, clamped to
    /// the line; a position inside a multibyte char falls back to the byte measure.
    fn carets(&self) -> Option<(usize, usize)> {
        let (text, s) = (&self.text, self.span);
        let byte = (s.col? as usize - 1).min(text.len());
        let start = text.get(..byte).map_or(byte, |t| t.chars().count());
        let len = s.len.map_or(1, |l| {
            let end = (byte + l as usize).min(text.len());
            text.get(byte..end)
                .map_or(l as usize, |t| t.chars().count())
        });
        Some((start, len.max(1)))
    }

    fn render(&self, p: &Paint, pad: &str) -> String {
        let bar = p.blue("|");
        let line = p.blue(&format!("{:>w$}", self.span.line, w = pad.len()));
        let mut out = format!("{pad} {bar}\n{line} {bar} {}\n", self.text);
        if let Some((start, len)) = self.carets() {
            let carets = p.level(self.level, &"^".repeat(len));
            let label = self
                .span
                .label
                .as_deref()
                .map_or(String::new(), |l| format!(" {}", p.level(self.level, l)));
            out += &format!("{pad} {bar} {}{carets}{label}\n", " ".repeat(start));
        }
        out + &format!("{pad} {bar}\n")
    }
}

/// The `= key: value` lines after the snippet: rule, help, notes, then the extra locations.
fn trailer_lines(d: &Diagnostic) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    if let Some(r) = &d.rule {
        lines.push((
            "rule",
            match &r.loc {
                Some(l) => format!("{} ({l})", r.name),
                None => r.name.clone(),
            },
        ));
    }
    if let Some(h) = &d.help {
        lines.push(("help", h.clone()));
    }
    for n in &d.notes {
        lines.push((
            "note",
            match &n.span {
                Some(s) => format!("{} ({})", n.text, loc(s)),
                None => n.text.clone(),
            },
        ));
    }
    if !d.spans.is_empty() {
        let shown: Vec<String> = d.spans.iter().take(SHOWN).map(loc).collect();
        let tail = match d.spans.len().saturating_sub(SHOWN) {
            0 => String::new(),
            more => format!(" and {more} more"),
        };
        let n = d.spans.len();
        lines.push((
            "note",
            format!("{n} more location(s): {}{tail}", shown.join(", ")),
        ));
    }
    lines
}

/// Render one diagnostic; `root` is where span files are read from for the snippet.
pub fn render(d: &Diagnostic, root: &Path, color: bool) -> String {
    let p = Paint(color);
    let mut out = format!(
        "{}: {}\n",
        p.level(d.level, &format!("{}[{}]", word(d.level), d.code)),
        d.message
    );
    let width = d.span.as_ref().map_or(1, |s| s.line.to_string().len());
    let pad = " ".repeat(width);
    if let Some(s) = &d.span {
        out += &format!("{pad}{} {}\n", p.blue("-->"), loc(s));
        if let Some(snippet) = Snippet::load(s, d.level, root) {
            out += &snippet.render(&p, &pad);
        }
    }
    let eq = p.blue("=");
    for (k, v) in trailer_lines(d) {
        out += &format!("{pad} {eq} {k}: {v}\n");
    }
    out
}

/// `1 warning`, `3 errors`.
pub fn plural(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}
/// All unaccepted diagnostics, then the summary line.
pub fn render_all(ds: &[Diagnostic], root: &Path, color: bool) -> String {
    let p = Paint(color);
    let mut out = String::new();
    for d in ds.iter().filter(|d| !d.accepted) {
        out += &render(d, root, color);
        out.push('\n');
    }
    let s = summarize(ds);
    let (lvl, word) = if s.errors > 0 {
        (Level::Error, "error")
    } else if s.warnings > 0 {
        (Level::Warn, "warning")
    } else {
        (Level::Warn, "ok")
    };
    let head = if word == "ok" {
        p.wrap("1;32", "ok")
    } else {
        p.level(lvl, word)
    };
    out += &format!(
        "{head}: {}, {}; {} accepted\n",
        plural(s.errors, "error"),
        plural(s.warnings, "warning"),
        s.accepted
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(span: Span) -> Diagnostic {
        Diagnostic {
            id: String::new(),
            level: Level::Warn,
            code: "c".into(),
            message: "m".into(),
            span: Some(span),
            spans: Vec::new(),
            rule: None,
            help: None,
            notes: Vec::new(),
            bricks: Vec::new(),
            brick_level: None,
            edges: Vec::new(),
            accepted: false,
            accepted_reason: None,
        }
    }

    fn span(col: Option<u32>, len: Option<u32>) -> Span {
        Span {
            file: "a.rs".into(),
            line: 1,
            col,
            len,
            label: Some("lbl".into()),
        }
    }

    /// (start, len) of the carets for `span` on a one-line text.
    fn carets(text: &str, span: Span) -> Option<(usize, usize)> {
        Snippet {
            text: text.into(),
            span: &span,
            level: Level::Warn,
        }
        .carets()
    }

    #[test]
    fn carets_sit_under_the_byte_column_and_count_chars() {
        // `fn` starts at 1-based byte col 4: 3 chars before, 2 carets.
        assert_eq!(carets("   fn name", span(Some(4), Some(2))), Some((3, 2)));
        assert_eq!(carets("abc", span(None, Some(2))), None);
    }

    #[test]
    fn multibyte_text_before_and_under_the_caret_counts_chars() {
        // "é" is 2 bytes. Col 5 is byte 4 (after "éé"): 2 chars before, " xy" is 3 chars.
        assert_eq!(carets("éé xy", span(Some(5), Some(3))), Some((2, 3)));
        // Col 6 is byte 5: 3 chars before; the 3 bytes are clamped to the line end, "xy".
        assert_eq!(carets("éé xy", span(Some(6), Some(3))), Some((3, 2)));
        // Col 2 is inside the first "é": falls back to the byte offset and byte length.
        assert_eq!(carets("éé xy", span(Some(2), Some(3))), Some((1, 3)));
    }

    #[test]
    fn col_beyond_the_line_clamps_to_its_end_with_one_caret() {
        assert_eq!(carets("ab", span(Some(50), Some(5))), Some((2, 1)));
        assert_eq!(carets("ab", span(Some(50), None)), Some((2, 1)));
    }

    #[test]
    fn missing_len_is_one_caret_and_zero_len_at_least_one() {
        assert_eq!(carets("abc", span(Some(2), None)), Some((1, 1)));
        assert_eq!(carets("abc", span(Some(2), Some(0))), Some((1, 1)));
    }

    #[test]
    fn more_locations_show_three_then_count_the_rest() {
        let mut d = diag(span(None, None));
        d.span = None;
        d.spans = (1..=5)
            .map(|l| Span {
                line: l,
                ..span(None, None)
            })
            .collect();
        let out = render(&d, Path::new("."), false);
        assert!(
            out.ends_with("= note: 5 more location(s): a.rs:1, a.rs:2, a.rs:3 and 2 more\n"),
            "{out}"
        );
        d.spans.truncate(3);
        let out = render(&d, Path::new("."), false);
        assert!(out.ends_with("a.rs:1, a.rs:2, a.rs:3\n"), "{out}");
    }
}
