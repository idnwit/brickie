//! Normalise a rust-analyzer SCIP index plus cargo metadata and the source snapshot into raw
//! graph rows. Pure: no I/O besides the `source` callback.

mod helpers;
pub mod layout;
pub mod modules;
mod normalise;
mod targets;
mod written;

pub use brickie_rustscan::UseItem;
pub use normalise::normalise;

/// (start_line, start_col, end_line, end_col), 0-based, end exclusive.
pub type Span = (u32, u32, u32, u32);

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

#[derive(Default)]
pub struct Graph {
    pub files: Vec<FileRow>,
    pub symbols: Vec<SymbolRow>,
    pub modules: Vec<ModuleRow>,
    pub occurrences: Vec<OccRow>,
    pub use_items: Vec<UseItem>,
}

pub fn span(r: &[i32]) -> Span {
    let u = |i: usize| r.get(i).copied().unwrap_or(0).max(0) as u32;
    if r.len() == 3 {
        (u(0), u(1), u(0), u(2))
    } else {
        (u(0), u(1), u(2), u(3))
    }
}
