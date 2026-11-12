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

/// Render one diagnostic; `root` is where span files are read from for the snippet.
pub fn render(d: &Diagnostic, root: &Path, color: bool) -> String {
    let p = Paint(color);
    let mut out = format!(
        "{}: {}\n",
        p.level(d.level, &format!("{}[{}]", word(d.level), d.code)),
        d.message
    );
    let src_line = |s: &Span| -> Option<String> {
        if s.line == 0 {
            return None;
        }
        let text = std::fs::read_to_string(root.join(&s.file)).ok()?;
        text.lines().nth(s.line as usize - 1).map(str::to_string)
    };
    let width = d
        .span
        .as_ref()
        .map_or(1, |s| s.line.to_string().len())
        .max(1);
    let pad = " ".repeat(width);
    if let Some(s) = &d.span {
        out += &format!("{pad}{} {}\n", p.blue("-->"), loc(s));
        if let Some(text) = src_line(s) {
            let bar = p.blue("|");
            out += &format!("{pad} {bar}\n");
            out += &format!("{} {bar} {text}\n", p.blue(&format!("{:>width$}", s.line)));
            if let Some(col) = s.col {
                let byte = (col as usize - 1).min(text.len());
                let start = text.get(..byte).map_or(byte, |t| t.chars().count());
                let len = s.len.map_or(1, |l| {
                    let end = (byte + l as usize).min(text.len());
                    text.get(byte..end)
                        .map_or(l as usize, |t| t.chars().count())
                });
                let carets = p.level(d.level, &"^".repeat(len.max(1)));
                let label = s
                    .label
                    .as_deref()
                    .map_or(String::new(), |l| format!(" {}", p.level(d.level, l)));
                out += &format!("{pad} {bar} {}{carets}{label}\n", " ".repeat(start));
            }
            out += &format!("{pad} {bar}\n");
        }
    }
    let eq = p.blue("=");
    let mut line = |k: &str, v: String| out += &format!("{pad} {eq} {k}: {v}\n");
    if let Some(r) = &d.rule {
        line(
            "rule",
            match &r.loc {
                Some(l) => format!("{} ({l})", r.name),
                None => r.name.clone(),
            },
        );
    }
    if let Some(h) = &d.help {
        line("help", h.clone());
    }
    for n in &d.notes {
        line(
            "note",
            match &n.span {
                Some(s) => format!("{} ({})", n.text, loc(s)),
                None => n.text.clone(),
            },
        );
    }
    if !d.spans.is_empty() {
        let shown: Vec<String> = d.spans.iter().take(SHOWN).map(loc).collect();
        let more = d.spans.len().saturating_sub(SHOWN);
        let tail = if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        };
        let n = d.spans.len();
        line(
            "note",
            format!("{n} more location(s): {}{tail}", shown.join(", ")),
        );
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
