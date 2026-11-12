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

/// Files of test code: tests/ and benches/ directories, files with an inner `cfg(test)`, and
/// files declared by a test-gated `mod x;` (transitively).
pub fn test_files(analyses: &BTreeMap<String, Analysis>) -> HashSet<String> {
    let seed = analyses
        .iter()
        .filter(|(p, a)| a.whole_file_test || p.split('/').any(|c| c == "tests" || c == "benches"))
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
                items: Vec::new(),
            },
        );
    }
    unreadable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cands(path: &str, name: &str) -> [String; 2] {
        mod_candidates(path, name)
    }

    fn an(decls: &[(&str, bool)], whole: bool) -> Analysis {
        Analysis {
            mod_decls: decls.iter().map(|&(n, t)| (n.to_string(), t)).collect(),
            whole_file_test: whole,
            ..Default::default()
        }
    }

    fn tests_of<const N: usize>(files: [(&str, Analysis); N]) -> Vec<String> {
        let analyses = files.into_iter().map(|(p, a)| (p.to_string(), a)).collect();
        let mut v: Vec<String> = test_files(&analyses).into_iter().collect();
        v.sort();
        v
    }

    #[test]
    fn candidates_for_crate_roots() {
        assert_eq!(cands("src/lib.rs", "a"), ["src/a.rs", "src/a/mod.rs"]);
        assert_eq!(cands("src/main.rs", "a"), ["src/a.rs", "src/a/mod.rs"]);
        assert_eq!(cands("src/x/mod.rs", "a"), ["src/x/a.rs", "src/x/a/mod.rs"]);
    }

    #[test]
    fn candidates_for_root_level_files() {
        assert_eq!(cands("lib.rs", "a"), ["a.rs", "a/mod.rs"]);
        assert_eq!(cands("util.rs", "a"), ["util/a.rs", "util/a/mod.rs"]);
    }

    #[test]
    fn candidates_for_nested_files() {
        assert_eq!(
            cands("src/run/files.rs", "a"),
            ["src/run/files/a.rs", "src/run/files/a/mod.rs"]
        );
    }

    // Replace `closure_keeps_empty_and_closed_sets`: no files, and a seed that adds nothing.
    #[test]
    fn test_files_keeps_empty_and_closed_sets() {
        assert!(tests_of([]).is_empty());
        // The seed declares a module that is not a known file: nothing to add.
        assert_eq!(
            tests_of([("tests/a.rs", an(&[("gone", false)], false))]),
            ["tests/a.rs"]
        );
    }

    // Replaces `closure_follows_chains`: a seed pulls a file that pulls another.
    #[test]
    fn test_files_follows_chains() {
        let got = tests_of([
            ("tests/a.rs", an(&[("b", false)], false)),
            ("tests/a/b.rs", an(&[("c", false)], false)),
            ("tests/a/b/c.rs", an(&[], false)),
            ("src/other.rs", an(&[], false)),
        ]);
        assert_eq!(got, ["tests/a.rs", "tests/a/b.rs", "tests/a/b/c.rs"]);
    }

    // Replaces `closure_terminates_on_cycles`: a `mod.rs` declaring `mod mod;` resolves to itself.
    #[test]
    fn test_files_terminates_on_cycles() {
        let got = tests_of([("tests/m/mod.rs", an(&[("mod", false), ("n", false)], false))]);
        assert_eq!(got, ["tests/m/mod.rs"]);
        let got = tests_of([
            ("src/x/mod.rs", an(&[("mod", true), ("n", true)], false)),
            ("src/x/n.rs", an(&[], false)),
        ]);
        assert_eq!(got, ["src/x/mod.rs", "src/x/n.rs"]);
    }

    #[test]
    fn analyze_all_reports_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ok.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.path().join("bin.rs"), b"fn b() {} // \xff\n").unwrap();
        let file = |path: &str| scip::FileRow {
            path: path.into(),
            ..Default::default()
        };
        let g = Graph {
            files: vec![file("ok.rs"), file("bin.rs"), file("gone.rs")],
            ..Default::default()
        };
        let (sources, analyses, unreadable) = analyze_all(&g, dir.path());
        assert_eq!(sources["bin.rs"], "fn b() {} // \u{fffd}\n");
        assert_eq!(analyses["bin.rs"].fns.len(), 1);
        assert_eq!(sources["gone.rs"], "");
        assert_eq!(unreadable, ["gone.rs"]);
    }
}
