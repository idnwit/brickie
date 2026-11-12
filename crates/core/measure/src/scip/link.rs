//! Pure `&mut Graph` phases over symbols: containment order, `symbol.parent`, `file.module`, then
//! crate-root names, module rows and `symbol.module`.

use super::helpers::{parent_path, parents};
use super::layout::Layout;
use super::{Graph, ModuleRow, Span};
use std::collections::HashMap;

/// Crate roots have no display name in SCIP; use the target name.
pub(super) fn name_crate_roots(g: &mut Graph, ws: &Layout) {
    for i in 0..g.symbols.len() {
        let s = &g.symbols[i];
        if s.module_path.as_deref() == Some("")
            && s.name.is_empty()
            && let Some(t) = g.files[s.file].target
        {
            g.symbols[i].name = ws.targets[t].name.clone();
        }
    }
}

/// `g.modules` (one row per module symbol) and `symbol.module`: a module's parent is found by
/// (target, parent path), any other symbol belongs to its nearest module ancestor, else the
/// module of its file.
pub(super) fn link_modules(g: &mut Graph) {
    // Files without a target share the key `(None, path)`; `or_insert` keeps the first module per
    // (target, path), so a duplicate path resolves to the first module symbol.
    let mut registry: HashMap<(Option<usize>, &str), usize> = HashMap::new();
    for (i, s) in g.symbols.iter().enumerate() {
        if let Some(p) = &s.module_path {
            registry
                .entry((g.files[s.file].target, p.as_str()))
                .or_insert(i);
        }
    }
    let module_of: Vec<Option<usize>> = g
        .symbols
        .iter()
        .enumerate()
        .map(|(i, s)| match &s.module_path {
            Some(p) if p.is_empty() => None,
            Some(p) => registry
                .get(&(g.files[s.file].target, parent_path(p)))
                .copied(),
            None => enclosing_module(g, i),
        })
        .collect();
    g.modules = (0..g.symbols.len())
        .filter(|&i| g.symbols[i].module_path.is_some())
        .map(|symbol| ModuleRow { symbol })
        .collect();
    for (s, m) in g.symbols.iter_mut().zip(module_of) {
        s.module = m;
    }
}

/// Nearest module among the `parent` chain of non-module symbol `i`, else its file's module.
fn enclosing_module(g: &Graph, i: usize) -> Option<usize> {
    let mut cur = g.symbols[i].parent;
    while let Some(c) = cur {
        if g.symbols[c].module_path.is_some() {
            break;
        }
        cur = g.symbols[c].parent;
    }
    cur.or(g.files[g.symbols[i].file].module)
}

/// Per file: (body span, symbol) sorted by start asc, end desc, then symbol id.
pub(super) fn file_defs(g: &Graph) -> Vec<Vec<(Span, usize)>> {
    let mut by_file: Vec<Vec<(Span, usize)>> = vec![Vec::new(); g.files.len()];
    for (i, s) in g.symbols.iter().enumerate() {
        by_file[s.file].push((s.body, i));
    }
    for defs in &mut by_file {
        defs.sort_by_key(|&(s, i)| ((s.0, s.1), std::cmp::Reverse((s.2, s.3)), i));
    }
    by_file
}

/// `symbol.parent`: only defs sorted before a symbol can be its parent, which keeps the
/// relation acyclic when two definitions share the same enclosing range.
pub(super) fn assign_parents(g: &mut Graph, file_defs: &[Vec<(Span, usize)>]) {
    for defs in file_defs {
        let found = parents(defs, |i| (g.symbols[i].range.0, g.symbols[i].range.1));
        for (i, parent) in found {
            g.symbols[i].parent = parent;
        }
    }
}

/// `file.module`: the module definition starting at (0, 0), else the outermost one.
pub(super) fn file_modules(g: &mut Graph, file_defs: &[Vec<(Span, usize)>]) {
    for (fi, defs) in file_defs.iter().enumerate() {
        let mut mods = defs
            .iter()
            .map(|&(_, i)| i)
            .filter(|&i| g.symbols[i].module_path.is_some())
            .peekable();
        let first = mods.peek().copied();
        let at_origin = mods.find(|&i| (g.symbols[i].body.0, g.symbols[i].body.1) == (0, 0));
        g.files[fi].module = at_origin.or(first);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scip::testutil::Builder;

    #[test]
    fn root_has_no_parent_children_find_theirs() {
        let mut b = Builder::default();
        let f = b.file("lib.rs");
        let root = b.module(f, "c::", "", None);
        let a = b.module(f, "c::a", "", None);
        let ab = b.module(f, "c::a::b", "", None);
        let mut g = b.g;
        g.modules.clear();
        link_modules(&mut g);
        assert_eq!(g.symbols[root].module, None);
        assert_eq!(g.symbols[a].module, Some(root));
        assert_eq!(g.symbols[ab].module, Some(a));
        let rows: Vec<usize> = g.modules.iter().map(|m| m.symbol).collect();
        assert_eq!(rows, vec![root, a, ab]);
    }

    #[test]
    fn duplicate_module_path_keeps_the_first_and_none_targets_share_a_key() {
        let mut b = Builder::default();
        let (f1, f2) = (b.file("a.rs"), b.file("b.rs"));
        let first = b.module(f1, "c::a", "", None);
        let _second = b.module(f2, "c::a", "", None);
        let child = b.module(f2, "c::a::x", "", None);
        let mut g = b.g;
        // Same target: the first `a` wins. Targetless files share one key as well.
        link_modules(&mut g);
        assert_eq!(g.symbols[child].module, Some(first));
        g.files[0].target = None;
        g.files[1].target = None;
        link_modules(&mut g);
        assert_eq!(g.symbols[child].module, Some(first));
    }

    #[test]
    fn item_belongs_to_nearest_module_ancestor_skipping_non_modules() {
        let mut b = Builder::default();
        let f = b.file("lib.rs");
        let root = b.module(f, "c::", "", None);
        let outer = b.item(f, "Outer", root);
        let inner = b.item(f, "inner", root);
        let mut g = b.g;
        g.symbols[inner].parent = Some(outer);
        g.symbols[outer].parent = Some(root);
        link_modules(&mut g);
        assert_eq!(g.symbols[inner].module, Some(root));
    }

    #[test]
    fn parentless_item_falls_back_to_its_file_module() {
        let mut b = Builder::default();
        let f = b.file("lib.rs");
        let root = b.module(f, "c::", "", None);
        let it = b.item(f, "x", root);
        let mut g = b.g;
        g.symbols[it].parent = None;
        g.symbols[it].module = None;
        link_modules(&mut g);
        assert_eq!(g.symbols[it].module, Some(root));
    }

    #[test]
    fn unnamed_crate_root_takes_the_target_name() {
        let mut b = Builder::default();
        let f = b.file("lib.rs");
        let root = b.module(f, "c::", "", None);
        b.g.symbols[root].name.clear();
        let ws = Layout {
            packages: Vec::new(),
            targets: vec![crate::scip::layout::Target {
                package: 0,
                name: "alpha".into(),
                src_path: "lib.rs".into(),
            }],
        };
        let mut g = b.g;
        name_crate_roots(&mut g, &ws);
        assert_eq!(g.symbols[root].name, "alpha");
    }

    /// One file with symbols of the given (body, is_module); ranges default to the body start.
    fn graph(syms: &[(Span, bool)]) -> Graph {
        let mut b = Builder::default();
        let f = b.file("f.rs");
        for (k, &(body, is_mod)) in syms.iter().enumerate() {
            let s = if is_mod {
                b.module(f, &format!("c::m{k}"), "", None)
            } else {
                let m = b.module(f, "c::", "", None);
                b.item(f, &format!("i{k}"), m)
            };
            b.g.symbols[s].body = body;
            b.g.symbols[s].range = (body.0, body.1, body.0, body.1 + 1);
        }
        b.g.files[0].module = None;
        for s in &mut b.g.symbols {
            s.parent = None;
        }
        b.g
    }

    #[test]
    fn defs_sort_outer_first_and_ties_by_id() {
        let mut g = graph(&[((2, 0, 3, 0), false), ((0, 0, 9, 0), false)]);
        g.symbols.retain(|s| s.module_path.is_none());
        g.symbols[0].file = 0;
        let d = file_defs(&g);
        let ids: Vec<usize> = d[0].iter().map(|&(_, i)| i).collect();
        assert_eq!(ids, vec![1, 0]);
    }

    #[test]
    fn equal_ranges_never_form_a_parent_cycle() {
        let mut g = graph(&[((0, 0, 9, 0), false), ((0, 0, 9, 0), false)]);
        g.symbols.retain(|s| s.module_path.is_none());
        let d = file_defs(&g);
        assign_parents(&mut g, &d);
        assert_eq!(g.symbols[0].parent, None);
        assert_eq!(g.symbols[1].parent, Some(0));
    }

    #[test]
    fn file_module_prefers_origin_then_outermost() {
        let mut g = graph(&[((1, 0, 9, 0), true), ((0, 0, 5, 0), true)]);
        let d = file_defs(&g);
        file_modules(&mut g, &d);
        assert_eq!(
            g.files[0].module,
            g.symbols.iter().position(|s| s.body.0 == 0)
        );
        g.symbols[1].body = (0, 4, 5, 0); // no module at (0, 0): the first by order
        let d = file_defs(&g);
        file_modules(&mut g, &d);
        assert_eq!(g.files[0].module, Some(1));
    }

    #[test]
    fn file_without_modules_has_none() {
        let mut g = graph(&[((0, 0, 9, 0), false)]);
        g.symbols.retain(|s| s.module_path.is_none());
        let d = file_defs(&g);
        file_modules(&mut g, &d);
        assert_eq!(g.files[0].module, None);
    }
}
