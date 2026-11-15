//! Per-file facts: functions with their complexity, test scope, and hashes of files the
//! compiler never saw.

use super::complexity::{self, Analysis};
use crate::scip::modules::ModuleIds;
use crate::scip::{self, Graph};
use brickie_facts::{Crate, FileData, Function, crate_dirs, sha256_hex, walk_rs};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

/// Sources and analyses by path, and the paths that could not be read.
pub type Analysed = (
    BTreeMap<String, String>,
    BTreeMap<String, Analysis>,
    Vec<String>,
);

pub fn scope(test: bool) -> &'static str {
    if test { "test" } else { "prod" }
}

/// Source text (lossy UTF-8) of `root/path`; a file that cannot be read at all is pushed to
/// `unreadable` and counts as empty.
fn read_lossy(root: &Path, path: &str, unreadable: &mut Vec<String>) -> String {
    match std::fs::read(root.join(path)) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => {
            unreadable.push(path.to_string());
            String::new()
        }
    }
}

/// Syntax analysis of every compiled file (complexity, test ranges, impl-trait positions), and
/// the files that could not be read.
pub fn analyze_all(g: &Graph, root: &Path) -> Analysed {
    let mut sources = BTreeMap::new();
    let mut analyses = BTreeMap::new();
    let mut unreadable = Vec::new();
    for f in &g.files {
        let src = read_lossy(root, &f.path, &mut unreadable);
        analyses.insert(f.path.clone(), complexity::analyze(&src));
        sources.insert(f.path.clone(), src);
    }
    (sources, analyses, unreadable)
}

/// True for a file in Cargo's own `tests/` or `benches/` directory of a package: such a segment
/// counts only when no `src` directory precedes it, so `src/tests/x.rs` (a plain module) is not
/// test code by path; there scope comes from `cfg(test)` on the `mod` chain or the item.
fn in_target_dir(path: &str) -> bool {
    for c in path.split('/') {
        match c {
            "src" => return false,
            "tests" | "benches" => return true,
            _ => {}
        }
    }
    false
}

/// Files of test code: tests/ and benches/ directories, files with an inner `cfg(test)`, and
/// files declared by a test-gated `mod x;` (transitively).
pub fn test_files(analyses: &BTreeMap<String, Analysis>) -> HashSet<String> {
    let seed = analyses
        .iter()
        .filter(|(p, a)| a.whole_file_test || in_target_dir(p))
        .map(|(p, _)| p.clone())
        .collect();
    // Fixpoint: stop once no file adds a new one (already-known files are ignored, so cycles end).
    let mut set: HashSet<String> = seed;
    loop {
        let added: Vec<String> = analyses
            .iter()
            .flat_map(|(path, a)| declared_files(analyses, &set, path, a))
            .collect();
        if added.is_empty() {
            return set;
        }
        set.extend(added);
    }
}

/// Known files that `path` declares as modules, and that are test code and not yet in `set`:
/// every `mod x;` of a file already in `set`, plus test-gated `mod x;` of any file.
fn declared_files(
    analyses: &BTreeMap<String, Analysis>,
    set: &HashSet<String>,
    path: &str,
    a: &Analysis,
) -> Vec<String> {
    let in_set = set.contains(path);
    a.mod_decls
        .iter()
        .filter(|(_, decl_test)| *decl_test || in_set)
        .flat_map(|(name, _)| mod_candidates(path, name))
        .filter(|c| analyses.contains_key(c) && !set.contains(c))
        .collect()
}

/// Paths where `mod name;` declared in file `path` may live: `<base>/name.rs` and
/// `<base>/name/mod.rs`, where `<base>` is the file's directory for `lib.rs`, `main.rs` and
/// `mod.rs`, and the directory plus the file stem otherwise (the stem alone at the crate root).
fn mod_candidates(path: &str, name: &str) -> [String; 2] {
    let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
    let stem = file.trim_end_matches(".rs");
    let base = if matches!(stem, "lib" | "main" | "mod") {
        dir.to_string()
    } else if dir.is_empty() {
        stem.to_string()
    } else {
        format!("{dir}/{stem}")
    };
    let join = |rest: String| {
        if base.is_empty() {
            rest
        } else {
            format!("{base}/{rest}")
        }
    };
    [join(format!("{name}.rs")), join(format!("{name}/mod.rs"))]
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
        let file_module = f.module.and_then(|s| ids.get(&s).cloned());
        let functions = a
            .fns
            .iter()
            .map(|m| {
                let module = file_modules[fi]
                    .iter()
                    .filter(|(b, _)| (b.0, b.1) <= m.start && m.start < (b.2, b.3))
                    .max_by_key(|(b, _)| (b.0, b.1))
                    .map(|(_, id)| (*id).clone())
                    .or_else(|| file_module.clone());
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
                module: file_module,
                functions,
                edges: Vec::new(),
                items: Vec::new(),
            },
        );
    }
    files
}

/// `.rs` files under crate directories that were not compiled (cfg-gated away, orphaned):
/// recorded with their hash only, so staleness does not report them as added. Returns the files
/// that could not be read (they are left out).
pub fn add_uncompiled(
    files: &mut BTreeMap<String, FileData>,
    crates: &[Crate],
    root: &Path,
) -> Vec<String> {
    let mut unreadable = Vec::new();
    let mut on_disk = BTreeSet::new();
    for d in crate_dirs(crates) {
        walk_rs(&root.join(d), root, &mut on_disk);
    }
    for path in on_disk {
        if files.contains_key(&path) {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            unreadable.push(path);
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let test = in_target_dir(&path);
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
                items: Vec::new(),
            },
        );
    }
    unreadable
}
#[cfg(test)]
mod tests;
