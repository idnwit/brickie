//! Rule evaluation over the cache: dependency rules, brick cycles, complexity limits and
//! unmapped modules. Each finding type implements [`Finding`] (see `diag.rs`).

use crate::cache::{Cache, Evidence};
use crate::config::{Brick, Config};
use crate::diag::{Finding, Span};
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

pub fn evaluate(cache: &Cache, cfg: &Config, root: &Path) -> Vec<Box<dyn Finding>> {
    let mut out: Vec<Box<dyn Finding>> = Vec::new();
    deps::deps(cache, cfg, &mut out);
    cycles::cycles(cache, cfg, &mut out);
    complexity::complexity(cache, cfg, root, &mut out);
    for m in &cache.modules {
        if !cache.bricks.module_brick.contains_key(&m.id) {
            out.push(Box::new(Unmapped {
                module: m.id.clone(),
                file: m.file.clone(),
                line: m.line,
            }));
        }
    }
    out
}

pub fn brick_of<'a>(cache: &Cache, cfg: &'a Config, module: &str) -> Option<&'a Brick> {
    let name = cache.bricks.module_brick.get(module)?;
    cfg.bricks.iter().find(|b| b.name == *name)
}

mod codes;
mod complexity;
mod cycles;
mod deps;
mod unmapped;

#[cfg(test)]
pub use codes::all as all_codes;
pub use codes::{DEPS_CYCLE, DEPS_DEFAULT, DEPS_DENY};
pub use complexity::limits as complexity_limits;
pub use deps::dep_subject;
use unmapped::Unmapped;
mod stale;
pub use stale::evaluate as evaluate_stale;
