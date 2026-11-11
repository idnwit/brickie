//! SCIP index -> [`Graph`] rows.

use super::helpers::*;
use super::targets::assign_targets;
use super::use_scan;
use super::written;
use super::*;
use crate::cache::sha256_hex;
use crate::measure::metadata::Workspace;
use ::scip::types::Index;
use std::collections::HashMap;

pub fn normalise(idx: &Index, ws: &Workspace, source: &dyn Fn(&str) -> Option<String>) -> Graph {
    let mut g = Graph::default();
    let info: HashMap<&str, &::scip::types::SymbolInformation> = idx
        .documents
        .iter()
        .flat_map(|d| d.symbols.iter())
        .filter(|s| !s.symbol.starts_with("local"))
        .map(|s| (s.symbol.as_str(), s))
        .collect();

    // Files, use items, definitions.
    let mut file_uses: Vec<Vec<UseItem>> = Vec::new();
    let mut sources: Vec<String> = Vec::new();
    let mut defs_by_scip: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut def_key: HashMap<(&str, usize), usize> = HashMap::new();
    for (fi, doc) in idx.documents.iter().enumerate() {
        let path = doc.relative_path.clone();
        let package = ws
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)))
            .max_by_key(|(_, p)| p.dir.len())
            .map(|(i, _)| i);
        let src = source(&path).unwrap_or_default();
        let uses = use_scan::use_items(&src);
        g.files.push(FileRow {
            sha256: sha256_hex(src.as_bytes()),
            lines: src.lines().count() as u32,
            path,
            package,
            target: None,
            module: None,
        });
        file_uses.push(uses);
        sources.push(src);
        for o in doc.occurrences.iter().filter(|o| is_def(o)) {
            if o.symbol.starts_with("local") || def_key.contains_key(&(o.symbol.as_str(), fi)) {
                continue;
            }
            let id = g.symbols.len();
            def_key.insert((o.symbol.as_str(), fi), id);
            defs_by_scip.entry(o.symbol.as_str()).or_default().push(id);
            let si = info.get(o.symbol.as_str());
            let range = span(&o.range);
            g.symbols.push(SymbolRow {
                scip: o.symbol.clone(),
                module: None,
                parent: None,
                name: si.map(|s| s.display_name.clone()).unwrap_or_default(),
                file: fi,
                range,
                body: if o.enclosing_range.is_empty() {
                    range
                } else {
                    span(&o.enclosing_range)
                },
                module_path: module_path(&o.symbol),
            });
        }
    }
    // Per-file definitions sorted for containment; symbol.parent_id.
    let mut file_defs: Vec<Vec<(Span, usize)>> = vec![Vec::new(); g.files.len()];
    for (i, s) in g.symbols.iter().enumerate() {
        file_defs[s.file].push((s.body, i));
    }
    for defs in &mut file_defs {
        defs.sort_by_key(|&(s, i)| ((s.0, s.1), std::cmp::Reverse((s.2, s.3)), i));
    }
    // Only defs sorted before a symbol can be its parent: keeps the parent
    // relation acyclic when two definitions share the same enclosing range.
    for defs in &file_defs {
        for (i, parent) in parents(defs, |i| (g.symbols[i].range.0, g.symbols[i].range.1)) {
            g.symbols[i].parent = parent;
        }
    }

    // File module: the module definition starting at (0, 0), else outermost.
    for (fi, defs) in file_defs.iter().enumerate() {
        let mods: Vec<usize> = defs
            .iter()
            .map(|&(_, i)| i)
            .filter(|&i| g.symbols[i].module_path.is_some())
            .collect();
        g.files[fi].module = mods
            .iter()
            .copied()
            .find(|&i| (g.symbols[i].body.0, g.symbols[i].body.1) == (0, 0))
            .or(mods.first().copied());
    }

    assign_targets(idx, ws, &mut g);
    // Crate roots have no display name in SCIP; use the target name.
    for s in g.symbols.iter_mut() {
        if s.module_path.as_deref() == Some("")
            && s.name.is_empty()
            && let Some(t) = g.files[s.file].target
        {
            s.name = ws.targets[t].name.clone();
        }
    }

    // Module rows: parent by (target, parent path); symbol.module_id.
    let mut registry: HashMap<(Option<usize>, &str), usize> = HashMap::new();
    for (i, s) in g.symbols.iter().enumerate() {
        if let Some(p) = &s.module_path {
            registry
                .entry((g.files[s.file].target, p.as_str()))
                .or_insert(i);
        }
    }
    let mut modules = Vec::new();
    let mut module_of = vec![None; g.symbols.len()];
    for (i, s) in g.symbols.iter().enumerate() {
        if let Some(p) = &s.module_path {
            let parent = if p.is_empty() {
                None
            } else {
                registry
                    .get(&(g.files[s.file].target, parent_path(p)))
                    .copied()
            };
            modules.push(ModuleRow { symbol: i });
            module_of[i] = parent;
        } else {
            let mut cur = s.parent;
            while let Some(c) = cur {
                if g.symbols[c].module_path.is_some() {
                    break;
                }
                cur = g.symbols[c].parent;
            }
            module_of[i] = cur.or(g.files[s.file].module);
        }
    }
    for (s, m) in g.symbols.iter_mut().zip(module_of) {
        s.module = m;
    }
    g.modules = modules;

    // Use items.
    let mut use_base = vec![0usize; g.files.len()];
    for (fi, uses) in file_uses.iter().enumerate() {
        use_base[fi] = g.use_items.len();
        for u in uses {
            g.use_items.push(u.clone());
        }
    }

    // Occurrences of workspace symbols.
    for (fi, doc) in idx.documents.iter().enumerate() {
        let uses = &file_uses[fi];
        let written = written::Written::new(&sources[fi]);
        let target = g.files[fi].target;
        for o in &doc.occurrences {
            if o.symbol.starts_with("local") {
                continue;
            }
            let range = span(&o.range);
            let pos = (range.0, range.1);
            let use_idx = if is_def(o) {
                None
            } else {
                let k = uses.partition_point(|u| u.start_pos <= pos);
                k.checked_sub(1).filter(|&k| uses[k].contains(pos.0, pos.1))
            };
            let Some(cands) = defs_by_scip.get(o.symbol.as_str()) else {
                continue; // external symbol: not part of the workspace graph
            };
            let symbol = if let Some(&id) = def_key.get(&(o.symbol.as_str(), fi)) {
                id
            } else {
                let same: Vec<usize> = cands
                    .iter()
                    .copied()
                    .filter(|&c| g.files[g.symbols[c].file].target == target)
                    .collect();
                let pool = if same.is_empty() { cands } else { &same };
                pool[0]
            };
            g.occurrences.push(OccRow {
                symbol,
                file: fi,
                range,
                roles: o.symbol_roles,
                container: innermost(&file_defs[fi], pos, Some(symbol)),
                import: use_idx.is_some(),
                use_item: use_idx.map(|k| use_base[fi] + k),
                written: written.at(range, use_idx.is_some()),
            });
        }
    }
    g
}
