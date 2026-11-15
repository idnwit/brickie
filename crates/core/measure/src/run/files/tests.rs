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
fn path_rule_is_only_for_cargo_target_dirs() {
    assert!(in_target_dir("tests/a.rs"));
    assert!(in_target_dir("crates/x/benches/b/main.rs"));
    assert!(!in_target_dir("src/tests/x.rs"));
    assert!(!in_target_dir("crates/x/src/tests.rs"));
    // An undeclared-by-cfg `pub mod tests;` stays prod; a gated one is test.
    let got = tests_of([
        ("src/lib.rs", an(&[("tests", false)], false)),
        ("src/tests.rs", an(&[("x", false)], false)),
        ("src/tests/x.rs", an(&[], false)),
    ]);
    assert!(got.is_empty());
    let got = tests_of([
        ("src/lib.rs", an(&[("tests", true)], false)),
        ("src/tests.rs", an(&[("x", false)], false)),
        ("src/tests/x.rs", an(&[], false)),
    ]);
    assert_eq!(got, ["src/tests.rs", "src/tests/x.rs"]);
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
