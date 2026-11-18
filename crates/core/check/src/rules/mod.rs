//! Rule evaluation: dependency rules, cycles, complexity, unmapped and stale.
//!
//! [`evaluate`] takes the measure cache, the config tree and the module-to-brick resolution and
//! returns one boxed [`Finding`] per violation. The brick is internal to the crate: the CLI and
//! the viewer reach it through the `check` facade. It does not read config files (that is
//! `config`, which also owns the allow/deny decision) and it does not render anything (that is
//! `diag`).
//!
//! # Usage
//!
//! ```text
//! let cfg = config::load(path)?;
//! let (resolved, issues) = config::resolve(&cfg, &cache.crates, &cache.modules, &ws_root);
//! let mut found = rules::evaluate(&cache, &cfg, &resolved, &ws_root);   // deps, cycles,
//!                                                                      // complexity, unmapped
//! found.extend(rules::evaluate_stale(&cache));                          // stale/file
//! found.extend(rules::evaluate_stale_measure(&cache, &cfg.measure));    // stale/measure
//! let diagnostics: Vec<_> = found.iter().map(|f| diag::diagnostic(f.as_ref())).collect();
//! ```
//!
//! # Rules
//!
//! | Code | Question it answers |
//! | --- | --- |
//! | `deps/unlinked` | Does a valid `[[link]]` cover this edge, lifted to the lowest common ancestor level? |
//! | `deps/private` | Does an edge into a composite reach a child it does not export? |
//! | `deps/cycle` | Do bricks of a level depend on each other in a cycle? |
//! | `complexity/<metric>` | Is a function over the tightest limit on its brick path? |
//! | `bricks/unmapped` | Does a module belong to no brick? |
//! | `stale/file`, `stale/measure` | Did files, workspace members or measure settings change since `measure`? |
//!
//! # Notes
//!
//! - A module edge is lifted to the level where its two ends are distinct siblings and judged
//!   there by `config::covering_link`; the decision is never re-implemented here.
//! - Staleness is decided from git candidates confirmed by sha256, or by hashing every cached
//!   file when git cannot help; it never re-measures.
//! - Also here: the design's own findings (`config/*` issues and `design/link-denied`) and the
//!   planned links the facade turns into the todo list.

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
mod design;
mod lift;
mod links;
mod private;
mod unmapped;

#[cfg(test)]
mod independent_tests;
#[cfg(test)]
mod tests;

pub use complexity::limits as complexity_limits;
pub use deps::dep_subject;
pub use design::{IssueFinding, LinkDenied, denied_text, named};
pub use links::{design_links, marked_links, planned_links};
use unmapped::Unmapped;
mod members;
mod stale;
pub use stale::{
    evaluate as evaluate_stale, evaluate_location as evaluate_stale_location,
    evaluate_measure as evaluate_stale_measure,
};
