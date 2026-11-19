//! Characterization of target assignment (through `normalise`): a lib and a bin share one source
//! directory, so a file's target is decided by which root file declares its `mod x;`.

use crate::scip::layout::{Layout, Package, Target};
use crate::scip::normalise;
use crate::scip::testutil::{DEF, document, index, occ};

const P: &str = "rust-analyzer cargo p 0.1.0 ";

fn layout() -> Layout {
    let target = |name: &str, src: &str| Target {
        package: 0,
        name: name.into(),
        src_path: src.into(),
    };
    Layout {
        packages: vec![Package { dir: "".into() }],
        targets: vec![target("p", "src/lib.rs"), target("p", "src/main.rs")],
    }
}

/// `file.target` by path. Documents are listed deepest first, so the result also pins that
/// files are resolved root-first regardless of their order in the index.
fn targets() -> Vec<(String, Option<usize>)> {
    let sym = |s: &str| format!("{P}{s}");
    let def = |path: &str, module: &str| {
        document(
            path,
            vec![occ(&sym(module), &[0, 0, 0], DEF, &[0, 0, 3, 0])],
            vec![],
        )
    };
    let with_ref = |mut d: ::scip::types::Document, module: &str, line: i32| {
        d.occurrences.push(occ(&sym(module), &[line, 4, 5], 0, &[]));
        d
    };
    let idx = index(vec![
        def("src/a/b.rs", "a/b/"),
        with_ref(def("src/a.rs", "a/"), "a/b/", 1),
        with_ref(def("src/main.rs", "crate/"), "a/", 1),
        def("src/lib.rs", "crate/"),
    ]);
    let (g, _) = normalise(&idx, &layout(), &|_| Some(String::new()));
    g.files.iter().map(|f| (f.path.clone(), f.target)).collect()
}

#[test]
fn shared_directory_follows_the_declaring_root() {
    let t = targets();
    let of = |p: &str| t.iter().find(|(f, _)| f == p).unwrap().1;
    assert_eq!(of("src/lib.rs"), Some(0));
    assert_eq!(of("src/main.rs"), Some(1));
    // `mod a;` is declared in main.rs only: the bin owns it and, in turn, `a/b.rs`.
    assert_eq!(of("src/a.rs"), Some(1));
    assert_eq!(of("src/a/b.rs"), Some(1));
}
