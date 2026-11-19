//! The measure cache (`.brickie/cache.json`): facts only, no rules.
//!
//! Per-file data is keyed by path so that one file's functions and outgoing edges can be
//! replaced without recomputing the others (incremental measure is not implemented yet).
//! Module-level edges are not stored; `Cache::edges` derives them from the per-file edges.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The cache layout version; a cache with another value must be measured again.
pub const SCHEMA_VERSION: u32 = 9;

/// The whole `.brickie/cache.json`: workspace shape plus per-file facts.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Cache {
    /// Equals [`SCHEMA_VERSION`] for a cache this build can read.
    pub schema_version: u32,
    /// The brickie version that measured.
    pub tool_version: String,
    /// The rust-analyzer version that made the index.
    pub rust_analyzer: String,
    /// How the index was made; `check` reports a cache whose settings differ from the config.
    #[serde(default)]
    pub measured: Measured,
    /// Absolute workspace root at measure time; all file paths are relative to it.
    pub workspace_root: String,
    /// `None` when the workspace is not a git repository.
    pub git: Option<Git>,
    pub crates: Vec<Crate>,
    pub modules: Vec<Module>,
    /// Per-file facts keyed by path relative to `workspace_root`.
    pub files: BTreeMap<String, FileData>,
}

/// The cargo settings of a measure run: the cache key besides the sources.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measured {
    /// `all` (every cargo feature); empty for a cache that predates the setting.
    pub features: String,
    /// Targets measured besides the host, sorted.
    pub targets: Vec<String>,
    /// Canonical directory of the config the run was made for; `check` reports a cache read
    /// from another directory (a copied or moved project). Empty: not judged.
    #[serde(default)]
    pub location: String,
}

impl Measured {
    /// All cargo features plus `targets` (already sorted and deduplicated by config loading).
    pub fn all(targets: Vec<String>) -> Measured {
        Measured {
            features: "all".into(),
            targets,
            location: String::new(),
        }
    }
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

/// The git state at measure time.
#[derive(Debug, Serialize, Deserialize)]
pub struct Git {
    /// `HEAD` commit hash.
    pub commit: String,
    /// The working tree had uncommitted changes.
    pub dirty: bool,
}

/// One cargo target. The library target is named after its package; other targets are
/// `package/kind/target`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Crate {
    /// Crate id, the prefix of its module ids.
    pub id: String,
    /// The cargo package name.
    pub package: String,
    pub version: String,
    /// The cargo target name.
    pub target: String,
    /// The cargo target kind (`lib`, `bin`, `test`, ...).
    pub kind: String,
    /// The target's root source file (`src/lib.rs`), relative to the workspace root.
    pub root: String,
    /// Package directory (holding `Cargo.toml`) relative to the workspace root, `""` for the
    /// root; `path:` brick members match against it.
    pub dir: String,
}

/// A Rust module (or crate root); its id is `<crate id>` for the crate root and
/// `<crate id>::a::b` otherwise.
#[derive(Debug, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    /// The id of the crate that contains it.
    #[serde(rename = "crate")]
    pub krate: String,
    /// The defining file, relative to the workspace root.
    pub file: String,
    /// 1-based line of the module definition.
    pub line: u32,
    /// The module's inner doc comment (`//!`, `#![doc]`) as markdown, `""` when undocumented.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
}

/// Everything measured about one source file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileData {
    /// Content hash ([`crate::sha256_hex`]); `check` compares it to find stale files.
    pub sha256: String,
    /// `false` for a `.rs` file under a crate directory that was not compiled (for example
    /// cfg-gated away): hashed so staleness can tell it from an added file, but it has no
    /// functions, edges or module.
    pub compiled: bool,
    /// Size in bytes.
    pub size: u64,
    pub lines: u32,
    /// `prod` or `test`.
    pub scope: String,
    /// Module defined by this file (its own module, not inline children).
    pub module: Option<String>,
    /// Functions with their complexity numbers.
    pub functions: Vec<Function>,
    /// Dependency edges whose evidence is in this file.
    pub edges: Vec<FileEdge>,
    /// Non-test items with a crate-wide or public visibility defined in this file: the
    /// candidates for a brick's declared interface.
    pub items: Vec<Item>,
}

/// A `pub` / `pub(crate)` item (fn, struct, enum, union, trait, type, const, static, or a
/// method / associated item of an inherent impl) from SCIP `SymbolInformation`. Trait items,
/// trait-impl items, fields, variants and modules are not recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Readable symbol, the same form as [`FileEdge::symbol`] (`model::Params::url`).
    pub symbol: String,
    /// Defining module (also the `to` of edges that use the item).
    pub module: String,
    /// `fn`, `method`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`.
    pub kind: String,
    /// Display name; `Type::name` for associated items.
    pub name: String,
    /// One-line signature as rust-analyzer renders it (`pub fn url(&self) -> String`).
    pub signature: String,
    /// The whole doc comment as markdown, `""` when undocumented; [`doc_summary`] gives its
    /// first paragraph.
    pub doc: String,
    /// 1-based line of the name.
    pub line: u32,
    /// `pub` or `pub(crate)`.
    pub vis: String,
    /// The module whose subtree can name the item, from its visibility, the visibility of the
    /// modules around it and `pub use` re-exports (the crate root for crate-wide items).
    pub visible_in: String,
    /// Reachable from the crate root of a library target as public API (`pub` through public
    /// modules or a `pub use` re-export).
    pub exported: bool,
}

/// The doc comment as markdown, trimmed; rust-analyzer's alias banner paragraph is dropped.
pub fn full_doc(doc: &str) -> String {
    let doc = doc.trim();
    match doc.strip_prefix("*This is the documentation for*") {
        Some(rest) => rest.split_once("\n\n").map_or("", |(_, b)| b).trim().into(),
        None => doc.into(),
    }
}

/// The first paragraph of a doc comment, on one line (the summary line of a listing).
pub fn doc_summary(doc: &str) -> String {
    doc.split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty())
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// One dependency edge between modules, with the site of its evidence in the file that holds it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileEdge {
    /// Module id of the using side.
    pub from: String,
    /// Module id of the used side.
    pub to: String,
    /// `uses`, `implements` or `reexports`.
    pub kind: String,
    /// `prod` or `test`.
    pub scope: String,
    pub line: u32,
    /// 1-based column range of the reference on `line`.
    pub col: u32,
    pub end_col: u32,
    /// Readable symbol of the used item.
    pub symbol: String,
    /// Path as written at the site, unresolved (`acme_engine::AudioEngine`, or a plain name).
    pub written: String,
}

/// One measured function and its complexity metrics (see the `complexity/*` codes).
#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    /// Qualified name, `Type::name` for methods.
    pub qname: String,
    /// Innermost module containing the function.
    pub module: Option<String>,
    pub line: u32,
    pub end_line: u32,
    pub lines: u32,
    pub cognitive: u32,
    pub cyclomatic: u32,
    pub nesting: u32,
    /// NPath.
    pub paths: u64,
    /// Parameters other than `self`.
    pub params: u32,
    /// `prod` or `test`.
    pub scope: String,
}

/// A module-level edge aggregated from [`FileEdge`]s by (from, to, kind, scope); see
/// [`Cache::edges`].
#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub scope: String,
    /// Every site that makes the edge.
    pub evidence: Vec<Evidence>,
}

/// One site behind an [`Edge`]: a reference at `line` between `col` and `end_col`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    pub symbol: String,
    /// Path as written at the site, unresolved.
    pub written: String,
}
