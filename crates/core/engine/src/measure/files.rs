//! Per-file facts: functions with their complexity, test scope, and hashes of files the
//! compiler never saw.

use super::complexity::{self, Analysis};
use super::modules::ModuleIds;
use super::scip::{self, Graph};
use crate::cache::{Crate, FileData, Function, crate_dirs, sha256_hex, walk_rs};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

pub fn scope(test: bool) -> &'static str {
    if test { "test" } else { "prod" }
}

/// Syntax analysis of every compiled file (complexity, test ranges, impl-trait positions).
pub fn analyze_all(
    g: &Graph,
    root: &Path,
) -> (BTreeMap<String, String>, BTreeMap<String, Analysis>) {
    let mut sources = BTreeMap::new();
    let mut analyses = BTreeMap::new();
    for f in &g.files {
        let src = std::fs::read_to_string(root.join(&f.path)).unwrap_or_default();
        analyses.insert(f.path.clone(), complexity::analyze(&src));
        sources.insert(f.path.clone(), src);
    }
    (sources, analyses)
}

/// Files of test code: tests/ and benches/ directories, files with an inner `cfg(test)`, and
/// files declared by a test-gated `mod x;` (transitively).
pub fn test_files(analyses: &BTreeMap<String, Analysis>) -> HashSet<String> {
    let mut set: HashSet<String> = analyses
        .iter()
        .filter(|(p, a)| a.whole_file_test || p.split('/').any(|c| c == "tests" || c == "benches"))
        .map(|(p, _)| p.clone())
        .collect();
    loop {
        let mut added = Vec::new();
        for (path, a) in analyses {
            let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
            let stem = file.trim_end_matches(".rs");
            let base = if matches!(stem, "lib" | "main" | "mod") {
                dir.to_string()
            } else if dir.is_empty() {
                stem.to_string()
            } else {
                format!("{dir}/{stem}")
            };
            let join = |rest: &str| {
                if base.is_empty() {
                    rest.to_string()
                } else {
                    format!("{base}/{rest}")
                }
            };
            for (name, decl_test) in &a.mod_decls {
                if !(*decl_test || set.contains(path)) {
                    continue;
                }
                for cand in [join(&format!("{name}.rs")), join(&format!("{name}/mod.rs"))] {
                    if analyses.contains_key(&cand) && !set.contains(&cand) {
                        added.push(cand);
                    }
                }
            }
        }
        if added.is_empty() {
            return set;
        }
        set.extend(added);
    }
}

/// Files the compiler saw: functions, module, scope and hash. Edges are filled in later.
pub fn compiled_files(
    g: &Graph,
    ids: &ModuleIds,
    sources: &BTreeMap<String, String>,
    analyses: &BTreeMap<String, Analysis>,
    test_set: &HashSet<String>,
) -> BTreeMap<String, FileData> {
    // Modules defined inside each file, for function -> module lookup.
    let mut file_modules: Vec<Vec<(scip::Span, &String)>> = vec![Vec::new(); g.files.len()];
    for (&sym, id) in ids {
        file_modules[g.symbols[sym].file].push((g.symbols[sym].body, id));
    }
    let mut files = BTreeMap::new();
    for (fi, f) in g.files.iter().enumerate() {
        let a = &analyses[&f.path];
        let file_test = test_set.contains(&f.path);
        let functions = a
            .fns
            .iter()
            .map(|m| {
                let module = file_modules[fi]
                    .iter()
                    .filter(|(b, _)| (b.0, b.1) <= m.start && m.start < (b.2, b.3))
                    .max_by_key(|(b, _)| (b.0, b.1))
                    .map(|(_, id)| (*id).clone())
                    .or_else(|| f.module.and_then(|s| ids.get(&s).cloned()));
                Function {
                    name: m.name.clone(),
                    qname: m.qname.clone(),
                    module,
                    line: m.line,
                    end_line: m.end_line,
                    lines: m.lines,
                    cognitive: m.cognitive,
                    cyclomatic: m.cyclomatic,
                    nesting: m.nesting,
                    paths: m.paths,
                    params: m.params,
                    scope: scope(file_test || m.test).to_string(),
                }
            })
            .collect();
        let size = sources[&f.path].len() as u64;
        files.insert(
            f.path.clone(),
            FileData {
                sha256: f.sha256.clone(),
                compiled: true,
                size,
                lines: f.lines,
                scope: scope(file_test).to_string(),
                module: f.module.and_then(|s| ids.get(&s).cloned()),
                functions,
                edges: Vec::new(),
            },
        );
    }
    files
}

/// `.rs` files under crate directories that were not compiled (cfg-gated away, orphaned):
/// recorded with their hash only, so staleness does not report them as added.
pub fn add_uncompiled(files: &mut BTreeMap<String, FileData>, crates: &[Crate], root: &Path) {
    let mut on_disk = BTreeSet::new();
    for d in crate_dirs(crates) {
        walk_rs(&root.join(d), root, &mut on_disk);
    }
    for path in on_disk {
        if files.contains_key(&path) {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let test = path.split('/').any(|c| c == "tests" || c == "benches");
        files.insert(
            path,
            FileData {
                sha256: sha256_hex(&bytes),
                compiled: false,
                size: bytes.len() as u64,
                lines: text.lines().count() as u32,
                scope: scope(test).to_string(),
                module: None,
                functions: Vec::new(),
                edges: Vec::new(),
            },
        );
    }
}
