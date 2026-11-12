//! Definition and references from the cached reference edges. The symbol at a position is
//! the reference edge whose site covers it, or the recorded item declared on that line under
//! the cursor. References are labelled with the using brick, other bricks first.

use super::read::read_confined;
use brickie_check::check::Loaded;
use brickie_check::config::level_name;
use codeview_port::{Location, Pos, Span};
use std::path::Path;

/// `(module, symbol)` of the symbol at `pos` in `path`.
fn target(l: &Loaded, path: &str, pos: Pos) -> Option<(String, String)> {
    let fd = l.cache.files.get(path)?;
    let edge = fd.edges.iter().find(|e| {
        e.kind != "reexports" && e.line == pos.line && e.col <= pos.col && pos.col <= e.end_col
    });
    if let Some(e) = edge {
        return Some((e.to.clone(), e.symbol.clone()));
    }
    let word = word_at(l, path, pos)?;
    fd.items
        .iter()
        .find(|i| i.line == pos.line && i.name.rsplit("::").next() == Some(word.as_str()))
        .map(|i| (i.module.clone(), i.symbol.clone()))
}

/// The identifier around byte column `pos.col` of line `pos.line`.
fn word_at(l: &Loaded, path: &str, pos: Pos) -> Option<String> {
    let bytes = read_confined(Path::new(&l.cache.workspace_root), path)?;
    let text = String::from_utf8_lossy(&bytes);
    let line = text.lines().nth((pos.line as usize).checked_sub(1)?)?;
    let at = (pos.col as usize).checked_sub(1)?.min(line.len());
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let start = line[..at]
        .char_indices()
        .rev()
        .take_while(|(_, c)| ident(*c))
        .last()
        .map_or(at, |(i, _)| i);
    let end = line[at..]
        .find(|c: char| !ident(c))
        .map_or(line.len(), |i| at + i);
    (start < end).then(|| line[start..end].to_string())
}

fn line_span(line: u32, col: u32, end_col: u32) -> Span {
    Span {
        start: Pos { line, col },
        end: Pos { line, col: end_col },
    }
}

/// The brick (level path) of a module, `(unmapped)` when it has none.
fn brick(l: &Loaded, module: &str) -> String {
    l.resolved
        .path(module)
        .map_or_else(|| "(unmapped)".to_string(), level_name)
}

/// Where the symbol at `pos` is declared: its recorded item, or else its module's file.
pub fn definition(l: &Loaded, path: &str, pos: Pos) -> Vec<Location> {
    let Some((module, symbol)) = target(l, path, pos) else {
        return Vec::new();
    };
    let items = l.cache.files.iter().flat_map(|(f, fd)| {
        fd.items
            .iter()
            .filter(|i| i.module == module && i.symbol == symbol)
            .map(move |i| (f, i))
    });
    let found: Vec<Location> = items
        .map(|(f, i)| Location {
            path: f.clone(),
            span: line_span(i.line, 1, 0),
            label: format!("{} \u{b7} {}", brick(l, &module), i.signature),
        })
        .collect();
    if !found.is_empty() {
        return found;
    }
    l.cache
        .modules
        .iter()
        .filter(|m| m.id == module)
        .map(|m| Location {
            path: m.file.clone(),
            span: line_span(m.line, 1, 0),
            label: format!(
                "{} \u{b7} module {module} (no recorded item {symbol})",
                brick(l, &module)
            ),
        })
        .collect()
}

/// Every reference site of the symbol at `pos`, other bricks first, then by brick and place.
pub fn references(l: &Loaded, path: &str, pos: Pos) -> Vec<Location> {
    let Some((module, symbol)) = target(l, path, pos) else {
        return Vec::new();
    };
    let home = brick(l, &module);
    let mut out: Vec<(bool, Location)> = Vec::new();
    for (f, fd) in &l.cache.files {
        for e in fd.edges.iter().filter(|e| e.kind != "reexports") {
            if e.to != module || e.symbol != symbol {
                continue;
            }
            let from = brick(l, &e.from);
            let same = from == home;
            let label = if same {
                format!("{from} (same brick)")
            } else {
                from
            };
            out.push((
                same,
                Location {
                    path: f.clone(),
                    span: line_span(e.line, e.col, e.end_col),
                    label,
                },
            ));
        }
    }
    out.sort_by(|a, b| {
        let key = |x: &(bool, Location)| {
            (
                x.0,
                x.1.label.clone(),
                x.1.path.clone(),
                x.1.span.start.line,
            )
        };
        key(a).cmp(&key(b))
    });
    out.dedup_by(|a, b| a.1 == b.1);
    out.into_iter().map(|x| x.1).collect()
}
