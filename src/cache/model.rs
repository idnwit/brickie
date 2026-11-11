//! The measure cache (`.brickie/cache.json`): facts only, no rules.
//!
//! Per-file data is keyed by path so that one file's functions and outgoing edges can be
//! replaced without recomputing the others (incremental measure is not implemented yet).
//! Module-level edges are not stored; `Cache::edges` derives them from the per-file edges.

use crate::config::Brick;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Serialize, Deserialize)]
pub struct Cache {
    pub schema_version: u32,
    pub tool_version: String,
    pub rust_analyzer: String,
    /// Absolute workspace root at measure time; all file paths are relative to it.
    pub workspace_root: String,
    /// `None` when the workspace is not a git repository.
    pub git: Option<Git>,
    pub bricks: Bricks,
    pub crates: Vec<Crate>,
    pub modules: Vec<Module>,
    pub files: BTreeMap<String, FileData>,
}

impl Cache {
    /// Module-level edges aggregated by (from, to, kind, scope), with evidence from the
    /// per-file edges. Derived on every call; the cache stores only per-file facts.
    pub fn edges(&self) -> Vec<Edge> {
        let mut agg: BTreeMap<(&str, &str, &str, &str), Vec<Evidence>> = BTreeMap::new();
        for (path, f) in &self.files {
            for e in &f.edges {
                agg.entry((&e.from, &e.to, &e.kind, &e.scope))
                    .or_default()
                    .push(Evidence {
                        file: path.clone(),
                        line: e.line,
                        col: e.col,
                        end_col: e.end_col,
                        symbol: e.symbol.clone(),
                        written: e.written.clone(),
                    });
            }
        }
        agg.into_iter()
            .map(|((from, to, kind, scope), evidence)| Edge {
                from: from.into(),
                to: to.into(),
                kind: kind.into(),
                scope: scope.into(),
                evidence,
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Git {
    pub commit: String,
    pub dirty: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bricks {
    /// sha256 of the parsed `[[brick]]` section.
    pub config_hash: String,
    pub defs: Vec<Brick>,
    /// Module id -> brick name; unmapped modules are absent.
    pub module_brick: BTreeMap<String, String>,
}

/// One cargo target. The library target is named after its package; other targets are
/// `package/kind/target`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Crate {
    pub id: String,
    pub package: String,
    pub version: String,
    pub target: String,
    pub kind: String,
    pub root: String,
}

/// Module id: `<crate id>` for the crate root, `<crate id>::a::b` otherwise.
#[derive(Debug, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    #[serde(rename = "crate")]
    pub krate: String,
    pub file: String,
    /// 1-based line of the module definition.
    pub line: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileData {
    pub sha256: String,
    /// `false` for a `.rs` file under a crate directory that was not compiled (for example
    /// cfg-gated away): hashed so staleness can tell it from an added file, but it has no
    /// functions, edges or module.
    pub compiled: bool,
    pub size: u64,
    pub lines: u32,
    pub scope: String,
    /// Module defined by this file (its own module, not inline children).
    pub module: Option<String>,
    pub functions: Vec<Function>,
    /// Dependency edges whose evidence is in this file.
    pub edges: Vec<FileEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub scope: String,
    pub line: u32,
    /// 1-based column range of the reference on `line`.
    pub col: u32,
    pub end_col: u32,
    pub symbol: String,
    /// Path as written at the site, unresolved (`acme_engine::AudioEngine`, or a plain name).
    pub written: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub qname: String,
    /// Innermost module containing the function.
    pub module: Option<String>,
    pub line: u32,
    pub end_line: u32,
    pub lines: u32,
    pub cognitive: u32,
    pub cyclomatic: u32,
    pub nesting: u32,
    pub paths: u64,
    pub params: u32,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub scope: String,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    pub symbol: String,
    /// Path as written at the site, unresolved.
    pub written: String,
}
