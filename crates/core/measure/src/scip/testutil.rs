//! Test builders: hand-made SCIP indexes and graphs, and a stable text rendering of a graph.

use crate::scip::modules::ModuleIds;
use crate::scip::*;
use ::scip::types::{Document, Index, Occurrence, Signature, SymbolInformation};
use std::fmt::Write;

/// SCIP definition role bit.
pub const DEF: i32 = 1;

/// Occurrence with a 3- or 4-element `range` and an optional enclosing range.
pub fn occ(symbol: &str, range: &[i32], roles: i32, enclosing: &[i32]) -> Occurrence {
    Occurrence {
        symbol: symbol.into(),
        range: range.to_vec(),
        symbol_roles: roles,
        enclosing_range: enclosing.to_vec(),
        ..Default::default()
    }
}

/// Symbol information with display name, doc comment and signature.
pub fn info(symbol: &str, name: &str, doc: &str, sig: &str) -> SymbolInformation {
    let mut s = SymbolInformation {
        symbol: symbol.into(),
        display_name: name.into(),
        ..Default::default()
    };
    if !doc.is_empty() {
        s.documentation.push(doc.into());
    }
    if !sig.is_empty() {
        s.signature_documentation = Some(Signature {
            text: sig.into(),
            ..Default::default()
        })
        .into();
    }
    s
}

pub fn document(
    path: &str,
    occurrences: Vec<Occurrence>,
    symbols: Vec<SymbolInformation>,
) -> Document {
    Document {
        relative_path: path.into(),
        occurrences,
        symbols,
        ..Default::default()
    }
}

pub fn index(documents: Vec<Document>) -> Index {
    Index {
        documents,
        ..Default::default()
    }
}

/// Every row of the graph, one per line, in a stable order (no hash iteration, no paths
/// beyond the relative ones given by the test).
pub fn render(g: &Graph) -> String {
    let mut o = String::new();
    for (i, f) in g.files.iter().enumerate() {
        let sha = &f.sha256[..8];
        let _ = writeln!(
            o,
            "file {i} {} pkg={:?} target={:?} sha={sha} lines={} module={:?}",
            f.path, f.package, f.target, f.lines, f.module
        );
    }
    for (i, s) in g.symbols.iter().enumerate() {
        let _ = writeln!(
            o,
            "sym {i} {:?} name={:?} file={} range={:?} body={:?} mp={:?} parent={:?} module={:?} sig={:?} doc={:?}",
            s.scip,
            s.name,
            s.file,
            s.range,
            s.body,
            s.module_path,
            s.parent,
            s.module,
            s.signature,
            s.doc
        );
    }
    let mods: Vec<usize> = g.modules.iter().map(|m| m.symbol).collect();
    let _ = writeln!(o, "modules {mods:?}");
    for (i, u) in g.use_items.iter().enumerate() {
        let _ = writeln!(
            o,
            "use {i} {:?}..{:?} pub={} vis={:?}",
            u.start_pos, u.end_pos, u.is_pub, u.vis
        );
    }
    for (i, c) in g.occurrences.iter().enumerate() {
        let _ = writeln!(
            o,
            "occ {i} sym={} file={} range={:?} roles={} container={:?} import={} use={:?} path_end={} written={:?}",
            c.symbol,
            c.file,
            c.range,
            c.roles,
            c.container,
            c.import,
            c.use_item,
            c.path_end,
            c.written
        );
    }
    o
}

/// Hand-built [`Graph`] plus its module ids, for tests of later stages.
#[derive(Default)]
pub struct Builder {
    pub g: Graph,
    pub ids: ModuleIds,
}

impl Builder {
    pub fn file(&mut self, path: &str) -> usize {
        self.g.files.push(FileRow {
            path: path.into(),
            package: Some(0),
            target: Some(0),
            sha256: String::new(),
            lines: 0,
            module: None,
        });
        self.g.files.len() - 1
    }

    /// A module symbol with module id `id`, declared in `file` (the file's module if it has
    /// none yet) with `sig` as its signature; `parent` is the enclosing module symbol.
    pub fn module(&mut self, file: usize, id: &str, sig: &str, parent: Option<usize>) -> usize {
        let path = id.split_once("::").map(|(_, p)| p.replace("::", "/"));
        let s = self.sym(file, id, parent, Some(path.unwrap_or_default()), sig);
        self.g.modules.push(ModuleRow { symbol: s });
        self.ids.insert(s, id.into());
        self.g.files[file].module.get_or_insert(s);
        s
    }

    /// A non-module item named `name` inside module symbol `module`.
    pub fn item(&mut self, file: usize, name: &str, module: usize) -> usize {
        self.sym(file, name, Some(module), None, "")
    }

    fn sym(
        &mut self,
        file: usize,
        name: &str,
        mo: Option<usize>,
        mp: Option<String>,
        sig: &str,
    ) -> usize {
        self.g.symbols.push(SymbolRow {
            scip: format!("s {name}"),
            module: mo,
            parent: None,
            name: name.rsplit("::").next().unwrap_or(name).into(),
            file,
            range: (0, 0, 0, 0),
            body: (0, 0, 0, 0),
            module_path: mp,
            signature: sig.into(),
            doc: String::new(),
        });
        self.g.symbols.len() - 1
    }

    pub fn use_item(&mut self, vis: &str) -> usize {
        self.g.use_items.push(UseItem {
            start: 0,
            end: 0,
            start_pos: (0, 0),
            end_pos: (0, 0),
            is_pub: vis.starts_with("pub"),
            vis: vis.into(),
        });
        self.g.use_items.len() - 1
    }

    /// A reference occurrence of `symbol` in `file` at `(row, col)`, 3 columns wide.
    pub fn occ(&self, file: usize, symbol: usize, row: u32, col: u32) -> OccRow {
        OccRow {
            symbol,
            file,
            range: (row, col, row, col + 3),
            roles: 0,
            container: None,
            import: false,
            use_item: None,
            written: String::new(),
            path_end: true,
        }
    }
}
