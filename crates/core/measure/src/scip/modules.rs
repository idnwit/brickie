//! Modules: one per (target, path), taken from the SCIP module symbols.

use super::{Graph, OccRow, SymbolRow};
use brickie_facts::Module;
use std::collections::{BTreeMap, HashMap};

/// Symbol index of a module symbol -> module id.
pub type ModuleIds = HashMap<usize, String>;

/// `crate/a/b` style SCIP module path to `a::b`.
fn colon_path(p: &str) -> String {
    p.replace('/', "::")
}

/// Prefers the definition that starts its own file when a module has several.
pub fn modules(g: &Graph, crate_ids: &[String]) -> (ModuleIds, Vec<Module>) {
    let mut module_id = ModuleIds::new();
    let mut modules: BTreeMap<String, (usize, &SymbolRow)> = BTreeMap::new();
    for row in &g.modules {
        let s = &g.symbols[row.symbol];
        let (Some(path), Some(t)) = (&s.module_path, g.files[s.file].target) else {
            continue;
        };
        let id = if path.is_empty() {
            crate_ids[t].clone()
        } else {
            format!("{}::{}", crate_ids[t], colon_path(path))
        };
        module_id.insert(row.symbol, id.clone());
        let starts_file = (s.body.0, s.body.1) == (0, 0);
        match modules.get(&id) {
            Some((_, old)) if (old.body.0, old.body.1) == (0, 0) || !starts_file => {}
            _ => {
                modules.insert(id, (row.symbol, s));
            }
        }
    }
    let list = modules
        .iter()
        .map(|(id, (_, s))| Module {
            id: id.clone(),
            krate: crate_ids[g.files[s.file].target.unwrap_or(0)].clone(),
            file: g.files[s.file].path.clone(),
            line: s.range.0 + 1,
        })
        .collect();
    (module_id, list)
}

/// Module of a symbol: itself if it is a module, else its enclosing module.
pub fn module_of<'a>(g: &Graph, ids: &'a ModuleIds, sym: usize) -> Option<&'a String> {
    ids.get(&sym)
        .or_else(|| g.symbols[sym].module.and_then(|m| ids.get(&m)))
}

/// Module an occurrence sits in: its container's module, or the file's own module when it has
/// no container.
pub fn from_module<'a>(g: &Graph, ids: &'a ModuleIds, o: &OccRow) -> Option<&'a String> {
    match o.container {
        Some(c) => module_of(g, ids, c),
        None => g.files[o.file].module.and_then(|m| ids.get(&m)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scip::testutil::Builder;

    #[test]
    fn from_module_prefers_container_then_file_module() {
        let mut b = Builder::default();
        let (lib, other) = (b.file("src/lib.rs"), b.file("src/m.rs"));
        let root = b.module(lib, "c", "", None);
        let m = b.module(other, "c::m", "mod m", Some(root));
        let x = b.item(other, "X", m);
        let plain = b.occ(other, x, 0, 0);
        assert_eq!(
            from_module(&b.g, &b.ids, &plain).map(String::as_str),
            Some("c::m")
        );
        let inside = OccRow {
            container: Some(root),
            ..b.occ(other, x, 0, 1)
        };
        assert_eq!(
            from_module(&b.g, &b.ids, &inside).map(String::as_str),
            Some("c")
        );
        b.g.files[other].module = None;
        assert_eq!(from_module(&b.g, &b.ids, &plain), None);
    }
}
