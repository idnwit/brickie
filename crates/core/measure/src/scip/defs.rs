//! Phase: one `SymbolRow` per (SCIP symbol, defining file), and lookup of the row an
//! occurrence refers to.

use super::helpers::{is_def, is_local, module_path};
use super::normalise::Builder;
use super::{Graph, SymbolRow, span};
use ::scip::types::{Occurrence, SymbolInformation};
use std::collections::HashMap;

/// Workspace definitions, by SCIP symbol and by (symbol, file); values index `g.symbols`.
#[derive(Default)]
pub(super) struct DefIndex<'a> {
    pub(super) by_scip: HashMap<&'a str, Vec<usize>>,
    pub(super) by_key: HashMap<(&'a str, usize), usize>,
}

impl DefIndex<'_> {
    /// The definition an occurrence of `scip` in `file` (of `target`) refers to: the one in the
    /// same file, else the first in the same target, else the first. `None` for external symbols.
    /// Candidates always share a package: `by_scip` is keyed by the full SCIP symbol (package and
    /// version), so `None == None` for untargeted files cannot select an unrelated def.
    pub(super) fn resolve(
        &self,
        g: &Graph,
        scip: &str,
        file: usize,
        target: Option<usize>,
    ) -> Option<usize> {
        let cands = self.by_scip.get(scip)?;
        if let Some(&id) = self.by_key.get(&(scip, file)) {
            return Some(id);
        }
        let same = cands
            .iter()
            .find(|&&c| g.files[g.symbols[c].file].target == target);
        Some(*same.unwrap_or(&cands[0]))
    }
}

fn symbol_row(o: &Occurrence, info: Option<&&SymbolInformation>, file: usize) -> SymbolRow {
    let range = span(&o.range);
    SymbolRow {
        scip: o.symbol.clone(),
        module: None,
        parent: None,
        name: info.map(|s| s.display_name.clone()).unwrap_or_default(),
        file,
        range,
        body: if o.enclosing_range.is_empty() {
            range
        } else {
            span(&o.enclosing_range)
        },
        module_path: module_path(&o.symbol),
        signature: info
            .and_then(|s| s.signature_documentation.as_ref())
            .map(|d| d.text.clone())
            .unwrap_or_default(),
        doc: info
            .and_then(|s| s.documentation.first())
            .cloned()
            .unwrap_or_default(),
    }
}

impl<'a> Builder<'a> {
    /// Locals are skipped; a symbol defined twice in one file keeps its first row (rust-analyzer
    /// does not index inactive `cfg` code, so a duplicate is not a `cfg` alternative).
    pub(super) fn collect_defs(&mut self) {
        let info: HashMap<&str, &SymbolInformation> = self
            .idx
            .documents
            .iter()
            .flat_map(|d| d.symbols.iter())
            .filter(|s| !is_local(&s.symbol))
            .map(|s| (s.symbol.as_str(), s))
            .collect();
        for (fi, doc) in self.idx.documents.iter().enumerate() {
            for o in doc.occurrences.iter().filter(|o| is_def(o.symbol_roles)) {
                let key = (o.symbol.as_str(), fi);
                if is_local(&o.symbol) || self.defs.by_key.contains_key(&key) {
                    continue;
                }
                let id = self.g.symbols.len();
                self.defs.by_key.insert(key, id);
                self.defs.by_scip.entry(key.0).or_default().push(id);
                let row = symbol_row(o, info.get(key.0), fi);
                self.g.symbols.push(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scip::testutil::Builder as Fixture;

    /// Files 0, 1, 2 in targets 0, 1, 1; `s` is defined in files 1 and 2 (symbols `a`, `b`).
    fn setup() -> (Graph, DefIndex<'static>, usize, usize) {
        let mut f = Fixture::default();
        let files: Vec<usize> = (0..3).map(|i| f.file(&format!("f{i}.rs"))).collect();
        let m = f.module(files[0], "c::", "", None);
        let (a, b) = (f.item(files[1], "s", m), f.item(files[2], "s", m));
        let mut g = f.g;
        for (file, target) in g.files.iter_mut().zip([0, 1, 1]) {
            file.target = Some(target);
        }
        let mut d = DefIndex::default();
        d.by_scip.insert("s", vec![a, b]);
        d.by_key.insert(("s", 1), a);
        d.by_key.insert(("s", 2), b);
        (g, d, a, b)
    }

    #[test]
    fn prefers_the_same_file() {
        let (g, d, a, b) = setup();
        assert_eq!(d.resolve(&g, "s", 1, Some(1)), Some(a));
        assert_eq!(d.resolve(&g, "s", 2, Some(1)), Some(b));
    }

    #[test]
    fn then_the_same_target_then_the_first() {
        let (mut g, d, a, _) = setup();
        // File 0 defines no `s` and is in target 0, where no candidate lives: the first.
        assert_eq!(d.resolve(&g, "s", 0, Some(0)), Some(a));
        // Candidates in targets 0 and 1: a target-1 reference skips the target-0 candidate.
        let mut d = d;
        d.by_scip.insert("s", vec![a, 2]);
        g.files[1].target = Some(0);
        assert_eq!(d.resolve(&g, "s", 0, Some(1)), Some(2));
    }

    #[test]
    fn a_symbol_defined_twice_in_one_file_keeps_the_first_row() {
        use crate::scip::layout::Layout;
        use crate::scip::normalise;
        use crate::scip::testutil::{DEF, document, index, occ};
        let doc = document(
            "a.rs",
            vec![
                occ("p 1 f().", &[0, 3, 4], DEF, &[0, 0, 0, 9]),
                occ("p 1 f().", &[2, 3, 4], DEF, &[2, 0, 2, 9]),
            ],
            vec![],
        );
        let ws = Layout {
            packages: Vec::new(),
            targets: Vec::new(),
        };
        let (g, _) = normalise(&index(vec![doc]), &ws, &|_| Some(String::new()));
        let rows: Vec<_> = g.symbols.iter().filter(|s| s.scip == "p 1 f().").collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].range.0, 0);
    }

    #[test]
    fn external_symbols_resolve_to_none() {
        let (g, d, ..) = setup();
        assert_eq!(d.resolve(&g, "ext", 0, Some(0)), None);
    }
}
