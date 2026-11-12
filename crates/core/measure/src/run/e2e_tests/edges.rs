//! Characterization of `add_edges`: one hand-built graph exercising every branch.

use crate::run::complexity::Analysis;
use crate::run::edges::add_edges;
use crate::scip::OccRow;
use crate::scip::testutil::Builder;
use brickie_facts::FileData;
use std::collections::{BTreeMap, HashSet};

fn file_data() -> FileData {
    FileData {
        sha256: String::new(),
        compiled: true,
        size: 0,
        lines: 0,
        scope: "prod".into(),
        module: None,
        functions: Vec::new(),
        edges: Vec::new(),
        items: Vec::new(),
    }
}

fn render(files: &BTreeMap<String, FileData>) -> String {
    let mut o = String::new();
    for (path, f) in files {
        o += &format!("{path}\n");
        for e in &f.edges {
            o += &format!(
                "  {} -> {} {} {} {}:{}-{} {:?} {:?}\n",
                e.from, e.to, e.kind, e.scope, e.line, e.col, e.end_col, e.symbol, e.written
            );
        }
    }
    o
}

#[test]
fn edges_by_branch() {
    let mut b = Builder::default();
    let (lib, a, bf, tf, gone) = (
        b.file("src/lib.rs"),
        b.file("src/a.rs"),
        b.file("src/b.rs"),
        b.file("src/tests.rs"),
        b.file("src/gone.rs"),
    );
    let root = b.module(lib, "c", "", None);
    let ma = b.module(a, "c::a", "pub mod a", Some(root));
    let mb = b.module(bf, "c::b", "pub mod b", Some(root));
    b.module(tf, "c::tests", "mod tests", Some(root));
    b.g.files[gone].module = Some(ma);
    let fa = b.item(a, "f", ma);
    let fa2 = b.item(a, "f2", ma);
    let fb = b.item(bf, "g", mb);
    b.g.symbols[fb].scip = "rust-analyzer cargo c 0.1.0 b/g().".into();
    let tr = b.item(bf, "Tr", mb);
    let orphan = b.item(bf, "Orphan", mb);
    b.g.symbols[orphan].module = None;
    let pub_use = b.use_item("pub");
    let plain_use = b.use_item("");

    let at = |f, s, r, c| b.occ(f, s, r, c);
    let wr = |o: OccRow, w: &str| OccRow {
        written: w.into(),
        ..o
    };
    let occs = vec![
        // Branches that produce no edge.
        OccRow {
            roles: 1,
            ..at(a, fb, 0, 0)
        },
        at(a, mb, 0, 1),
        OccRow {
            container: Some(fa),
            ..at(a, fa2, 0, 2)
        },
        at(a, orphan, 0, 3),
        OccRow {
            container: Some(orphan),
            ..at(a, fb, 0, 4)
        },
        // No container: the file's module is the source.
        wr(at(a, fb, 3, 4), "b::g"),
        wr(at(a, fb, 3, 4), "b::g"),
        // Container, multi-line range: end column is col + 1.
        OccRow {
            container: Some(fa),
            range: (5, 8, 6, 2),
            ..at(a, fb, 5, 8)
        },
        // Imports: pub use is a re-export, a private use is a plain use.
        OccRow {
            import: true,
            use_item: Some(pub_use),
            ..at(a, fb, 10, 4)
        },
        OccRow {
            import: true,
            use_item: Some(plain_use),
            ..at(a, fb, 11, 4)
        },
        // Trait impl position: implements, but not when it is an import.
        at(a, tr, 7, 10),
        OccRow {
            import: true,
            use_item: Some(plain_use),
            ..at(a, tr, 8, 10)
        },
        // `pub use` item without an import flag is a plain use.
        OccRow {
            use_item: Some(pub_use),
            ..at(a, fb, 12, 4)
        },
        // Test scope: a test file, and a position inside a test range.
        at(tf, fb, 1, 1),
        at(a, fb, 20, 0),
        // Reverse direction from another file; a file missing from the output map.
        at(bf, fa, 2, 2),
        at(gone, fb, 0, 0),
    ];
    b.g.occurrences = occs;

    let mut analyses: BTreeMap<String, Analysis> =
        b.g.files
            .iter()
            .map(|f| (f.path.clone(), Analysis::default()))
            .collect();
    let an = analyses.get_mut("src/a.rs").unwrap();
    an.impl_traits = vec![(7, 10), (8, 10)];
    an.test_ranges = vec![((19, 0), (30, 0))];
    // `impl_traits` positions are per file: a position in another file never matches.
    analyses.get_mut("src/b.rs").unwrap().impl_traits = vec![(3, 4)];
    let tests: HashSet<String> = ["src/tests.rs".to_string()].into();
    let mut files: BTreeMap<String, FileData> =
        b.g.files
            .iter()
            .filter(|f| f.path != "src/gone.rs")
            .map(|f| (f.path.clone(), file_data()))
            .collect();
    add_edges(&mut files, &b.g, &b.ids, &analyses, &tests);
    assert_eq!(render(&files), EXPECTED.trim_start());
}

const EXPECTED: &str = r#"
src/a.rs
  c::a -> c::b implements prod 8:11-14 "Tr" ""
  c::a -> c::b reexports prod 11:5-8 "b::g" ""
  c::a -> c::b uses prod 4:5-8 "b::g" "b::g"
  c::a -> c::b uses prod 6:9-9 "b::g" ""
  c::a -> c::b uses prod 9:11-14 "Tr" ""
  c::a -> c::b uses prod 12:5-8 "b::g" ""
  c::a -> c::b uses prod 13:5-8 "b::g" ""
  c::a -> c::b uses test 21:1-4 "b::g" ""
src/b.rs
  c::b -> c::a uses prod 3:3-6 "f" ""
src/lib.rs
src/tests.rs
  c::tests -> c::b uses test 2:2-5 "b::g" ""
"#;
