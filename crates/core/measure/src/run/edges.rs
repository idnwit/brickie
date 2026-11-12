//! Dependency edges between modules, from SCIP occurrences.

use super::complexity::Analysis;
use super::files::scope;
use crate::scip::Graph;
use crate::scip::modules::{ModuleIds, module_of};
use brickie_facts::{FileData, FileEdge};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Readable symbol: its descriptors joined with `::` (`shape::Shape`, `util::double`).
fn symbol_name(scip: &str, fallback: &str) -> String {
    match ::scip::symbol::parse_symbol(scip) {
        Ok(s) => {
            let v: Vec<&str> = s
                .descriptors
                .iter()
                .map(|d| d.name.as_str())
                .filter(|n| *n != "crate")
                .collect();
            if v.is_empty() {
                fallback.to_string()
            } else {
                v.join("::")
            }
        }
        Err(_) => fallback.to_string(),
    }
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
        if o.roles & 1 == 1 {
            continue; // definitions are not dependencies
        }
        let sym = &g.symbols[o.symbol];
        if sym.module_path.is_some() {
            continue; // module path segments (`a` in `a::f`): the used item carries the edge
        }
        let Some(to) = module_of(g, ids, o.symbol) else {
            continue;
        };
        let from = match o.container {
            Some(c) => module_of(g, ids, c),
            None => g.files[o.file].module.and_then(|m| ids.get(&m)),
        };
        let Some(from) = from else { continue };
        if from == to {
            continue;
        }
        let f = &g.files[o.file];
        let (row, col) = (o.range.0, o.range.1);
        let pub_use = o.use_item.is_some_and(|u| g.use_items[u].is_pub);
        let kind = if o.import && pub_use {
            "reexports"
        } else if !o.import && implements.contains(&(o.file, row, col)) {
            "implements"
        } else {
            "uses"
        };
        let test = test_set.contains(&f.path) || analyses[&f.path].is_test_at((row, col));
        per_file[o.file].insert(FileEdge {
            from: from.clone(),
            to: to.clone(),
            kind: kind.into(),
            scope: scope(test).into(),
            line: row + 1,
            col: col + 1,
            end_col: if o.range.2 == row {
                o.range.3 + 1
            } else {
                col + 1
            },
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
