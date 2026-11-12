//! SCIP index -> [`Graph`] rows: a builder owning the graph under construction and the indexes
//! the phases share. The graph-only phases are in `link.rs`, definitions in `defs.rs`.

use super::defs::DefIndex;
use super::helpers::{innermost, is_def, is_local};
use super::layout::Layout;
use super::link::{assign_parents, file_defs, file_modules, link_modules, name_crate_roots};
use super::targets::assign_targets;
use super::written::Written;
use super::{FileRow, Graph, OccRow, Span, UseItem, span};
use ::scip::types::{Index, Occurrence};
use brickie_facts::sha256_hex;
use brickie_rustscan as use_scan;
use std::ops::Range;

/// The graph, plus the relative paths whose source `source` could not provide (recorded as
/// empty: sha256 of the empty string, zero lines).
pub fn normalise(
    idx: &Index,
    ws: &Layout,
    source: &dyn Fn(&str) -> Option<String>,
) -> (Graph, Vec<String>) {
    let mut b = Builder::new(idx, ws);
    b.add_files(source);
    b.collect_defs();
    b.index_tree();
    assign_targets(idx, ws, &mut b.g);
    name_crate_roots(&mut b.g, ws);
    link_modules(&mut b.g);
    b.add_occurrences();
    (b.g, b.unreadable)
}

/// The graph being built plus the per-file and per-definition indexes the phases share.
pub(super) struct Builder<'a> {
    pub(super) idx: &'a Index,
    pub(super) ws: &'a Layout,
    pub(super) g: Graph,
    /// Per file (same order as `idx.documents`): its source text.
    sources: Vec<String>,
    /// Per file: the range of its `use` items in `g.use_items`.
    file_uses: Vec<Range<usize>>,
    /// Paths whose source could not be read.
    unreadable: Vec<String>,
    pub(super) defs: DefIndex<'a>,
    /// Per file: (body span, symbol) sorted for containment queries.
    file_defs: Vec<Vec<(Span, usize)>>,
}

/// Package owning `path`: the one with the longest directory prefix (an empty dir owns all).
fn package_of(ws: &Layout, path: &str) -> Option<usize> {
    ws.packages
        .iter()
        .enumerate()
        .filter(|(_, p)| p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)))
        .max_by_key(|(_, p)| p.dir.len())
        .map(|(i, _)| i)
}

/// Index of the `use` item (sorted by start) containing `pos`, if any.
fn use_at(uses: &[UseItem], pos: (u32, u32)) -> Option<usize> {
    let k = uses.partition_point(|u| u.start_pos <= pos);
    k.checked_sub(1).filter(|&k| uses[k].contains(pos.0, pos.1))
}

impl<'a> Builder<'a> {
    fn new(idx: &'a Index, ws: &'a Layout) -> Self {
        Builder {
            idx,
            ws,
            g: Graph::default(),
            sources: Vec::new(),
            file_uses: Vec::new(),
            unreadable: Vec::new(),
            defs: DefIndex::default(),
            file_defs: Vec::new(),
        }
    }

    /// One `FileRow` per document, and every file's `use` items appended to `g.use_items` in
    /// document order. Unreadable files hash and count as the empty string, and are listed in
    /// `unreadable`.
    fn add_files(&mut self, source: &dyn Fn(&str) -> Option<String>) {
        for doc in &self.idx.documents {
            let path = doc.relative_path.clone();
            let src = source(&path).unwrap_or_else(|| {
                self.unreadable.push(path.clone());
                String::new()
            });
            let start = self.g.use_items.len();
            self.g.use_items.extend(use_scan::use_items(&src));
            self.file_uses.push(start..self.g.use_items.len());
            self.g.files.push(FileRow {
                sha256: sha256_hex(src.as_bytes()),
                lines: src.lines().count() as u32,
                package: package_of(self.ws, &path),
                path,
                target: None,
                module: None,
            });
            self.sources.push(src);
        }
    }

    /// Containment order per file, then `symbol.parent` and `file.module`.
    fn index_tree(&mut self) {
        self.file_defs = file_defs(&self.g);
        assign_parents(&mut self.g, &self.file_defs);
        file_modules(&mut self.g, &self.file_defs);
    }

    /// One `OccRow` per occurrence of a workspace symbol; occurrences of locals and of external
    /// symbols are skipped.
    fn add_occurrences(&mut self) {
        for (fi, doc) in self.idx.documents.iter().enumerate() {
            let written = Written::new(&self.sources[fi]);
            for o in doc.occurrences.iter().filter(|o| !is_local(&o.symbol)) {
                if let Some(row) = self.occurrence(fi, o, &written) {
                    self.g.occurrences.push(row);
                }
            }
        }
    }

    fn occurrence(&self, fi: usize, o: &Occurrence, written: &Written) -> Option<OccRow> {
        let range = span(&o.range);
        let pos = (range.0, range.1);
        let uses = self.file_uses[fi].clone();
        let use_idx = if is_def(o.symbol_roles) {
            None
        } else {
            use_at(&self.g.use_items[uses.clone()], pos).map(|k| uses.start + k)
        };
        let target = self.g.files[fi].target;
        let symbol = self.defs.resolve(&self.g, &o.symbol, fi, target)?;
        Some(OccRow {
            symbol,
            file: fi,
            range,
            roles: o.symbol_roles,
            // For a def occurrence this may name a child (see `innermost`); defs are dropped later.
            container: innermost(&self.file_defs[fi], pos, Some(symbol)),
            import: use_idx.is_some(),
            use_item: use_idx,
            written: written.at(range, use_idx.is_some()),
            path_end: written.is_path_end(range),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scip::layout::Package;

    fn ws(dirs: &[&str]) -> Layout {
        Layout {
            packages: dirs.iter().map(|d| Package { dir: (*d).into() }).collect(),
            targets: Vec::new(),
        }
    }

    #[test]
    fn longest_dir_wins() {
        let ws = ws(&["crates/a", "crates/a/sub"]);
        assert_eq!(package_of(&ws, "crates/a/sub/x.rs"), Some(1));
        assert_eq!(package_of(&ws, "crates/a/x.rs"), Some(0));
    }

    #[test]
    fn empty_dir_owns_everything_but_loses_to_longer() {
        let ws = ws(&["", "crates/a"]);
        assert_eq!(package_of(&ws, "src/x.rs"), Some(0));
        assert_eq!(package_of(&ws, "crates/a/x.rs"), Some(1));
    }

    #[test]
    fn prefix_must_end_at_a_separator() {
        assert_eq!(package_of(&ws(&["crates/a"]), "crates/ab/x.rs"), None);
        assert_eq!(package_of(&ws(&[]), "x.rs"), None);
    }

    fn item(start: (u32, u32), end: (u32, u32)) -> UseItem {
        UseItem {
            start: 0,
            end: 0,
            start_pos: start,
            end_pos: end,
            is_pub: false,
            vis: String::new(),
        }
    }

    #[test]
    fn use_at_boundaries() {
        let uses = [item((1, 0), (1, 10)), item((3, 0), (4, 5))];
        assert_eq!(use_at(&uses, (0, 0)), None);
        assert_eq!(use_at(&uses, (1, 0)), Some(0));
        assert_eq!(use_at(&uses, (1, 9)), Some(0));
        assert_eq!(use_at(&uses, (2, 0)), None);
        assert_eq!(use_at(&uses, (3, 3)), Some(1));
        assert_eq!(use_at(&uses, (9, 0)), None);
        assert_eq!(use_at(&[], (0, 0)), None);
    }
}
