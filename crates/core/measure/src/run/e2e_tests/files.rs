//! Characterization of `test_files`: seeds, module-file resolution and the transitive closure.

use crate::run::complexity::Analysis;
use crate::run::files::test_files;
use std::collections::BTreeMap;

/// Analysis with the given `mod x;` declarations (name, test-gated) and whole-file flag.
fn an(decls: &[(&str, bool)], whole: bool) -> Analysis {
    Analysis {
        mod_decls: decls.iter().map(|&(n, t)| (n.to_string(), t)).collect(),
        whole_file_test: whole,
        ..Default::default()
    }
}

fn run<const N: usize>(files: [(&str, Analysis); N]) -> Vec<String> {
    let analyses: BTreeMap<String, Analysis> =
        files.into_iter().map(|(p, a)| (p.to_string(), a)).collect();
    let mut v: Vec<String> = test_files(&analyses).into_iter().collect();
    v.sort();
    v
}

#[test]
fn nothing_gated_means_no_test_files() {
    let got = run([
        ("src/lib.rs", an(&[("a", false)], false)),
        ("src/a.rs", an(&[], false)),
    ]);
    assert!(got.is_empty());
}

#[test]
fn seeds_are_tests_benches_and_whole_file_gated() {
    let got = run([
        ("src/lib.rs", an(&[], false)),
        ("tests/it.rs", an(&[], false)),
        ("crates/x/tests/deep/more.rs", an(&[], false)),
        ("benches/b.rs", an(&[], false)),
        ("src/w.rs", an(&[], true)),
        ("src/tests_helper.rs", an(&[], false)),
    ]);
    assert_eq!(
        got,
        [
            "benches/b.rs",
            "crates/x/tests/deep/more.rs",
            "src/w.rs",
            "tests/it.rs"
        ]
    );
}

#[test]
fn module_files_resolve_from_each_root_kind() {
    let got = run([
        // lib.rs: children sit next to it; a gated decl pulls both candidate forms.
        (
            "src/lib.rs",
            an(&[("t", true), ("both", true), ("clean", false)], false),
        ),
        ("src/t.rs", an(&[("deep", false), ("deep2", false)], false)),
        ("src/t/deep.rs", an(&[], false)),
        ("src/t/deep2/mod.rs", an(&[], false)),
        ("src/both.rs", an(&[], false)),
        ("src/both/mod.rs", an(&[], false)),
        ("src/clean.rs", an(&[], false)),
        // mod.rs: children sit in its directory.
        ("src/b/mod.rs", an(&[("c", true)], false)),
        ("src/b/c.rs", an(&[], false)),
        // main.rs at the crate root directory: base is empty.
        ("main.rs", an(&[("m", true)], false)),
        ("m.rs", an(&[], false)),
        // A plain file at the root: children sit in a directory named after it.
        ("x.rs", an(&[("y", true)], false)),
        ("x/y.rs", an(&[], false)),
        // Nested crate dir with a lib root, and a declaration whose file does not exist.
        (
            "other/src/lib.rs",
            an(&[("g", true), ("missing", true)], false),
        ),
        ("other/src/g.rs", an(&[], false)),
    ]);
    assert_eq!(got, EXPECTED);
}

#[test]
fn closure_follows_ungated_decls_of_test_files() {
    let got = run([
        // A seed in tests/ pulls the modules it declares, without a gate on the decl.
        ("tests/it.rs", an(&[("common", false)], false)),
        ("tests/it/common.rs", an(&[("util", false)], false)),
        ("tests/it/common/util.rs", an(&[], false)),
        // A whole-file test module pulls its children, and so does a nested chain.
        ("src/w.rs", an(&[("sub", false)], true)),
        ("src/w/sub.rs", an(&[("subsub", false)], false)),
        ("src/w/sub/subsub.rs", an(&[], false)),
        // An ungated, unrelated chain stays out.
        ("src/p.rs", an(&[("q", false)], false)),
        ("src/p/q.rs", an(&[], false)),
    ]);
    assert_eq!(got, CLOSURE);
}

const EXPECTED: &[&str] = &[
    "m.rs",
    "other/src/g.rs",
    "src/b/c.rs",
    "src/both.rs",
    "src/both/mod.rs",
    "src/t.rs",
    "src/t/deep.rs",
    "src/t/deep2/mod.rs",
    "x/y.rs",
];
const CLOSURE: &[&str] = &[
    "src/w.rs",
    "src/w/sub.rs",
    "src/w/sub/subsub.rs",
    "tests/it.rs",
    "tests/it/common.rs",
    "tests/it/common/util.rs",
];
