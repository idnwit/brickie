//! rust-analyzer SCIP index into file, symbol, module and occurrence rows.
//!
//! [`normalise`] turns a decoded SCIP index, the cargo [`layout::Layout`] (package directories
//! and target roots) and a source reader into a [`Graph`]: file rows with hashes, symbol rows
//! with signature and doc, module rows and occurrence rows (definitions and references). It
//! is pure apart from the `source` callback, and it neither measures complexity nor decides
//! brick membership.
//!
//! # Usage
//!
//! ```text
//! let idx = scip::types::Index::parse_from_bytes(&bytes)?;
//! let layout = layout::Layout { packages, targets };       // from cargo metadata
//! let (graph, unreadable) = normalise(&idx, &layout, &|path| std::fs::read_to_string(path).ok());
//! let (module_ids, modules) = modules::modules(&graph, &crate_ids);
//! ```
//!
//! # Key items
//!
//! - [`normalise`]: index to [`Graph`]; also returns the paths whose source could not be read.
//! - [`Graph`], [`FileRow`], [`SymbolRow`], [`ModuleRow`], [`OccRow`]: the rows.
//! - [`modules::modules`]: module ids and the cache's module list.
//! - [`descriptor_names`]: the name segments of a SCIP symbol.
//! - [`UseItem`]: a parsed `use` item, re-exported from `brickie-rustscan`.
//!
//! # Notes
//!
//! A file whose source cannot be provided is recorded as empty (sha256 of the empty string, zero
//! lines) and listed in the returned paths. Spans are `(start_line, start_col, end_line,
//! end_col)`, 0-based, end exclusive.

mod defs;
mod helpers;
pub mod layout;
mod link;
pub mod modules;
mod normalise;
mod targets;
#[cfg(test)]
pub(crate) mod testutil;
mod written;

pub use brickie_rustscan::UseItem;
pub use helpers::descriptor_names;
pub use normalise::normalise;

/// (start_line, start_col, end_line, end_col), 0-based, end exclusive.
pub type Span = (u32, u32, u32, u32);

#[derive(Default)]
pub struct FileRow {
    pub path: String,
    pub package: Option<usize>,
    pub target: Option<usize>,
    pub sha256: String,
    pub lines: u32,
    pub module: Option<usize>,
}

pub struct SymbolRow {
    pub scip: String,
    pub module: Option<usize>,
    pub parent: Option<usize>,
    pub name: String,
    pub file: usize,
    pub range: Span,
    pub body: Span,
    /// `Some(path)` when the symbol is a module (`''` = crate root).
    pub module_path: Option<String>,
    /// rust-analyzer's signature (`pub fn f(x: u32) -> u32`), `""` when absent.
    pub signature: String,
    /// The doc comment (markdown), `""` when absent.
    pub doc: String,
}

pub struct ModuleRow {
    pub symbol: usize,
}

pub struct OccRow {
    pub symbol: usize,
    pub file: usize,
    pub range: Span,
    pub roles: i32,
    pub container: Option<usize>,
    pub import: bool,
    pub use_item: Option<usize>,
    /// Path as written at the site (qualifiers or full `use` path), unresolved.
    pub written: String,
    /// The reference ends its path (not followed by `::`, except `::*`): the leaf of a `use`
    /// or the module of a glob import, rather than a qualifier segment.
    pub path_end: bool,
}

impl OccRow {
    /// A definition site (as opposed to a reference).
    pub fn is_def(&self) -> bool {
        helpers::is_def(self.roles)
    }
}

#[derive(Default)]
pub struct Graph {
    pub files: Vec<FileRow>,
    pub symbols: Vec<SymbolRow>,
    pub modules: Vec<ModuleRow>,
    pub occurrences: Vec<OccRow>,
    pub use_items: Vec<UseItem>,
}

fn span(r: &[i32]) -> Span {
    let u = |i: usize| r.get(i).copied().unwrap_or(0).max(0) as u32;
    if r.len() == 3 {
        (u(0), u(1), u(0), u(2))
    } else {
        (u(0), u(1), u(2), u(3))
    }
}
