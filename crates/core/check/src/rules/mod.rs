//! Rule evaluation over the cache: dependency rules, brick cycles, complexity limits and
//! unmapped modules. Each finding type implements [`Finding`] (see `diag.rs`).

use crate::config::{Brick, Config, Resolved};
use crate::diag::{Finding, Span};
use brickie_facts::{Cache, Evidence};
use std::path::Path;

fn span(e: &Evidence, label: Option<String>) -> Span {
    Span {
        file: e.file.clone(),
        line: e.line,
        col: Some(e.col),
        len: Some(e.end_col.saturating_sub(e.col).max(1)),
        label,
    }
}

fn tags(b: &Brick) -> String {
    if b.tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", b.tags.join(", "))
    }
}

/// All findings of the brick tree `cfg` over the cache, with modules resolved by `res`.
pub fn evaluate(cache: &Cache, cfg: &Config, res: &Resolved, root: &Path) -> Vec<Box<dyn Finding>> {
    let mut out: Vec<Box<dyn Finding>> = Vec::new();
    deps::deps(cache, cfg, res, &mut out);
    cycles::cycles(cache, cfg, res, &mut out);
    complexity::complexity(cache, cfg, res, root, &mut out);
    for m in &cache.modules {
        if res.path(&m.id).is_none() {
            out.push(Box::new(Unmapped {
                module: m.id.clone(),
                file: m.file.clone(),
                line: m.line,
            }));
        }
    }
    out
}

mod complexity;
mod cycles;
mod deps;
mod lift;
mod private;
mod unmapped;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod via_tests;

pub use complexity::limits as complexity_limits;
pub use deps::dep_subject;
use unmapped::Unmapped;
mod stale;
pub use stale::evaluate as evaluate_stale;
