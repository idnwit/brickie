//! Assigning each file to the cargo target (lib, bin, ...) it belongs to.

use super::{Graph, helpers::*};
use crate::measure::metadata::Workspace;
use ::scip::types::Index;
use std::collections::{HashMap, HashSet};

/// `file.target_id`: walk from each target's root file along the module tree,
/// using file layout and the `mod x;` reference in the parent file to
/// disambiguate targets that share a source directory (lib + bin).
pub fn assign_targets(idx: &Index, ws: &Workspace, g: &mut Graph) {
    let refs: Vec<HashSet<&str>> = idx
        .documents
        .iter()
        .map(|d| {
            d.occurrences
                .iter()
                .filter(|o| !is_def(o) && o.symbol.ends_with('/'))
                .map(|o| o.symbol.as_str())
                .collect()
        })
        .collect();
    let file_path = |fi: usize| {
        g.files[fi]
            .module
            .and_then(|m| g.symbols[m].module_path.clone())
    };
    let mut order: Vec<(usize, String)> = (0..g.files.len())
        .filter_map(|fi| file_path(fi).map(|p| (fi, p)))
        .collect();
    order.sort_by_key(|(_, p)| {
        if p.is_empty() {
            0
        } else {
            p.split('/').count()
        }
    });
    // (target, module path) -> file that defines it.
    let mut placed: HashMap<(usize, String), usize> = HashMap::new();
    for (fi, mpath) in order {
        let path = g.files[fi].path.clone();
        let package = g.files[fi].package;
        let in_pkg = || {
            ws.targets
                .iter()
                .enumerate()
                .filter(move |(_, t)| Some(t.package) == package)
        };
        let target = if mpath.is_empty() {
            in_pkg().find(|(_, t)| t.src_path == path).map(|(i, _)| i)
        } else {
            let scip = g.symbols[g.files[fi].module.unwrap_or(0)].scip.as_str();
            let cands: Vec<usize> = in_pkg()
                .filter(|(_, t)| {
                    expected_module_path(&t.src_path, &path).as_deref() == Some(&mpath)
                })
                .map(|(i, _)| i)
                .collect();
            let declared = |t: usize| {
                placed
                    .get(&(t, parent_path(&mpath).to_string()))
                    .is_some_and(|&pf| refs[pf].contains(scip))
            };
            cands
                .iter()
                .copied()
                .find(|&t| declared(t))
                .or(cands.first().copied())
                .or_else(|| in_pkg().map(|(i, _)| i).find(|&t| declared(t)))
        };
        g.files[fi].target = target;
        if let Some(t) = target {
            for s in g.symbols.iter().filter(|s| s.file == fi) {
                if let Some(p) = &s.module_path {
                    placed.entry((t, p.clone())).or_insert(fi);
                }
            }
        }
    }
}
