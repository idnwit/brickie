//! Dependency edges between modules, from SCIP occurrences.

use super::complexity::Analysis;
use super::files::scope;
use crate::scip::descriptor_names;
use crate::scip::modules::{ModuleIds, from_module, module_of};
use crate::scip::{Graph, OccRow};
use brickie_facts::{FileData, FileEdge};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Readable symbol: its descriptors joined with `::` (`shape::Shape`, `util::double`).
pub(super) fn symbol_name(scip: &str, fallback: &str) -> String {
    match descriptor_names(scip) {
        Some(v) if !v.is_empty() => v.join("::"),
        _ => fallback.to_string(),
    }
}

/// `reexports` for a pub `use` import, else `implements` for a non-import impl-trait site,
/// else `uses`; an import never counts as `implements`.
fn edge_kind(import: bool, pub_use: bool, implements: bool) -> &'static str {
    if import && pub_use {
        "reexports"
    } else if !import && implements {
        "implements"
    } else {
        "uses"
    }
}

/// 1-based end column: the span's end when it stays on the start line, else the 1-based start.
fn end_col(range: (u32, u32, u32, u32)) -> u32 {
    if range.2 == range.0 {
        range.3 + 1
    } else {
        range.1 + 1
    }
}

/// `(from, to)` modules of a dependency occurrence; `None` for definitions, module path
/// segments, unresolved ends and self edges. No container means the file's own module.
fn endpoints<'a>(g: &Graph, ids: &'a ModuleIds, o: &OccRow) -> Option<(&'a String, &'a String)> {
    if o.is_def() || g.symbols[o.symbol].module_path.is_some() {
        return None;
    }
    let to = module_of(g, ids, o.symbol)?;
    let from = from_module(g, ids, o)?;
    (from != to).then_some((from, to))
}

/// Fills `edges` of every compiled file in `files`.
pub fn add_edges(
    files: &mut BTreeMap<String, FileData>,
    g: &Graph,
    ids: &ModuleIds,
    analyses: &BTreeMap<String, Analysis>,
    test_set: &HashSet<String>,
) {
    let implements: HashSet<(usize, u32, u32)> = g
        .files
        .iter()
        .enumerate()
        .flat_map(|(fi, f)| {
            analyses[&f.path]
                .impl_traits
                .iter()
                .map(move |&(r, c)| (fi, r, c))
        })
        .collect();
    let mut per_file: Vec<BTreeSet<FileEdge>> = vec![BTreeSet::new(); g.files.len()];
    for o in &g.occurrences {
        let Some((from, to)) = endpoints(g, ids, o) else {
            continue;
        };
        let (path, sym) = (&g.files[o.file].path, &g.symbols[o.symbol]);
        let (row, col) = (o.range.0, o.range.1);
        let pub_use = o.use_item.is_some_and(|u| g.use_items[u].is_pub);
        let kind = edge_kind(o.import, pub_use, implements.contains(&(o.file, row, col)));
        let test = test_set.contains(path) || analyses[path].is_test_at((row, col));
        per_file[o.file].insert(FileEdge {
            from: from.clone(),
            to: to.clone(),
            kind: kind.into(),
            scope: scope(test).into(),
            line: row + 1,
            col: col + 1,
            end_col: end_col(o.range),
            symbol: symbol_name(&sym.scip, &sym.name),
            written: o.written.clone(),
        });
    }
    for (fi, edges) in per_file.into_iter().enumerate() {
        if let Some(fd) = files.get_mut(&g.files[fi].path) {
            fd.edges = edges.into_iter().collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_table() {
        for import in [false, true] {
            for pub_use in [false, true] {
                for implements in [false, true] {
                    let want = if import && pub_use {
                        "reexports"
                    } else if !import && implements {
                        "implements"
                    } else {
                        "uses"
                    };
                    assert_eq!(edge_kind(import, pub_use, implements), want);
                }
            }
        }
        assert_eq!(edge_kind(true, false, true), "uses");
        assert_eq!(edge_kind(false, false, true), "implements");
        assert_eq!(edge_kind(true, true, true), "reexports");
    }

    #[test]
    fn end_col_same_line_and_multiline() {
        assert_eq!(end_col((3, 4, 3, 9)), 10);
        assert_eq!(end_col((3, 4, 5, 9)), 5);
    }
}
