//! Assigning each file to the cargo target (lib, bin, ...) it belongs to.

use super::layout::{Layout, Target};
use super::{Graph, helpers::*};
use ::scip::types::Index;
use std::collections::{HashMap, HashSet};

/// Module symbols each document refers to (`mod x;` and `use` references, not definitions).
fn mod_refs(idx: &Index) -> Vec<HashSet<&str>> {
    idx.documents
        .iter()
        .map(|d| {
            d.occurrences
                .iter()
                .filter(|o| !is_def(o.symbol_roles) && o.symbol.ends_with('/'))
                .map(|o| o.symbol.as_str())
                .collect()
        })
        .collect()
}

/// `(file, module path)` of every file that has a module, parents before children (stable, so
/// files of one depth keep their order).
fn order_by_depth(g: &Graph) -> Vec<(usize, String)> {
    let mut order: Vec<(usize, String)> = (0..g.files.len())
        .filter_map(|fi| {
            let m = g.files[fi].module?;
            Some((fi, g.symbols[m].module_path.clone()?))
        })
        .collect();
    order.sort_by_key(|(_, p)| {
        if p.is_empty() {
            0
        } else {
            p.split('/').count()
        }
    });
    order
}

/// Decides targets file by file, remembering which file defined each (target, module path).
struct Placer<'a> {
    ws: &'a Layout,
    refs: Vec<HashSet<&'a str>>,
    /// (target, module path) -> file that defines it.
    placed: HashMap<(usize, String), usize>,
}

impl Placer<'_> {
    /// Targets of the file's package, with their index.
    fn in_pkg(&self, package: Option<usize>) -> impl Iterator<Item = (usize, &Target)> {
        self.ws
            .targets
            .iter()
            .enumerate()
            .filter(move |(_, t)| Some(t.package) == package)
    }

    /// Whether target `t`'s file for the parent module refers to the module symbol `scip`.
    fn declared(&self, t: usize, mpath: &str, scip: &str) -> bool {
        self.placed
            .get(&(t, parent_path(mpath).to_string()))
            .is_some_and(|&pf| self.refs[pf].contains(scip))
    }

    fn target_of(&self, g: &Graph, fi: usize, mpath: &str) -> Option<usize> {
        let (path, package) = (&g.files[fi].path, g.files[fi].package);
        if mpath.is_empty() {
            return self
                .in_pkg(package)
                .find(|(_, t)| &t.src_path == path)
                .map(|(i, _)| i);
        }
        let scip = g.symbols[g.files[fi].module.unwrap_or(0)].scip.as_str();
        let cands: Vec<usize> = self
            .in_pkg(package)
            .filter(|(_, t)| expected_module_path(&t.src_path, path).as_deref() == Some(mpath))
            .map(|(i, _)| i)
            .collect();
        let declared = |t: usize| self.declared(t, mpath, scip);
        cands
            .iter()
            .copied()
            .find(|&t| declared(t))
            .or(cands.first().copied())
            .or_else(|| self.in_pkg(package).map(|(i, _)| i).find(|&t| declared(t)))
    }

    /// Records that file `fi` defines every module it holds, in target `t`.
    fn place(&mut self, g: &Graph, fi: usize, t: usize) {
        for s in g.symbols.iter().filter(|s| s.file == fi) {
            if let Some(p) = &s.module_path {
                self.placed.entry((t, p.clone())).or_insert(fi);
            }
        }
    }
}

/// `file.target_id`: walk from each target's root file along the module tree,
/// using file layout and the `mod x;` reference in the parent file to
/// disambiguate targets that share a source directory (lib + bin).
pub fn assign_targets(idx: &Index, ws: &Layout, g: &mut Graph) {
    let mut placer = Placer {
        ws,
        refs: mod_refs(idx),
        placed: HashMap::new(),
    };
    for (fi, mpath) in order_by_depth(g) {
        let target = placer.target_of(g, fi, &mpath);
        g.files[fi].target = target;
        if let Some(t) = target {
            placer.place(g, fi, t);
        }
    }
}
