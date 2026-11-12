//! Characterization of `add_items`: one hand-built graph exercising every skip guard and both
//! owner kinds.

use crate::run::complexity::Analysis;
use crate::run::items::add_items;
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
        for i in &f.items {
            o += &format!(
                "  {} {} {} {:?} {:?} line={} vis={} in={} exported={} sym={}\n",
                i.module,
                i.kind,
                i.name,
                i.signature,
                i.doc,
                i.line,
                i.vis,
                i.visible_in,
                i.exported,
                i.symbol
            );
        }
    }
    o
}

/// (name, scip tail, signature, file, module, row)
type Row<'a> = (&'a str, &'a str, &'a str, usize, Option<usize>, u32);

#[test]
fn items_by_branch() {
    let mut b = Builder::default();
    let (lib, a, tf) = (
        b.file("src/lib.rs"),
        b.file("src/a.rs"),
        b.file("src/tests.rs"),
    );
    let root = b.module(lib, "c", "", None);
    let ma = b.module(a, "c::a", "pub mod a", Some(root));
    let mt = b.module(tf, "c::tests", "mod tests", Some(root));
    let rows: [Row; 14] = [
        (
            "free",
            "p 0.1.0 free().",
            "pub fn free()\n",
            lib,
            Some(root),
            1,
        ),
        (
            "hidden",
            "p 0.1.0 hidden().",
            "pub fn hidden()",
            a,
            Some(ma),
            2,
        ),
        (
            "crate_fn",
            "p 0.1.0 crate_fn().",
            "pub(crate) fn crate_fn()",
            a,
            Some(ma),
            3,
        ),
        (
            "private",
            "p 0.1.0 private().",
            "fn private()",
            a,
            Some(ma),
            4,
        ),
        (
            "sup",
            "p 0.1.0 sup().",
            "pub(super) fn sup()",
            a,
            Some(ma),
            5,
        ),
        (
            "in_test_file",
            "p 0.1.0 in_test_file().",
            "pub fn in_test_file()",
            tf,
            Some(mt),
            6,
        ),
        (
            "in_test_range",
            "p 0.1.0 in_test_range().",
            "pub fn in_test_range()",
            a,
            Some(ma),
            25,
        ),
        (
            "no_module",
            "p 0.1.0 no_module().",
            "pub fn no_module()",
            a,
            None,
            7,
        ),
        (
            "unknown_module",
            "p 0.1.0 unknown_module().",
            "pub fn unknown_module()",
            a,
            Some(99),
            8,
        ),
        ("T", "p 0.1.0 T#", "pub trait T", lib, Some(root), 9),
        (
            "member",
            "p 0.1.0 T#member().",
            "pub fn member()",
            lib,
            Some(root),
            10,
        ),
        (
            "new",
            "p 0.1.0 impl#[S]new().",
            "pub fn new()",
            lib,
            Some(root),
            11,
        ),
        (
            "tr_impl",
            "p 0.1.0 impl#[S][T]tr_impl().",
            "pub fn tr_impl()",
            lib,
            Some(root),
            12,
        ),
        ("bad", "not a symbol", "pub fn bad()", lib, Some(root), 13),
    ];
    for (name, scip, sig, file, module, row) in rows {
        let s = b.item(file, name, root);
        let sym = &mut b.g.symbols[s];
        sym.scip = format!("rust-analyzer cargo {scip}");
        if scip == "not a symbol" {
            sym.scip = scip.into();
        }
        sym.signature = sig.into();
        sym.doc = "Doc.\n\nMore.".into();
        sym.module = module;
        sym.range = (row, 0, row, 3);
    }
    // A pub field-like signature has no item keyword.
    let f = b.item(lib, "field", root);
    b.g.symbols[f].signature = "pub name: String".into();
    b.g.symbols[f].scip = "rust-analyzer cargo p 0.1.0 S#name.".into();

    let mut analyses: BTreeMap<String, Analysis> =
        b.g.files
            .iter()
            .map(|f| (f.path.clone(), Analysis::default()))
            .collect();
    analyses.get_mut("src/a.rs").unwrap().test_ranges = vec![((20, 0), (30, 0))];
    let tests: HashSet<String> = ["src/tests.rs".to_string()].into();
    let mut files: BTreeMap<String, FileData> =
        b.g.files
            .iter()
            .map(|f| (f.path.clone(), file_data()))
            .collect();
    add_items(
        &mut files,
        &b.g,
        &b.ids,
        &["c".to_string()],
        &analyses,
        &tests,
    );
    assert_eq!(render(&files), EXPECTED.trim_start());
}

const EXPECTED: &str = r#"
src/a.rs
  c::a fn hidden "pub fn hidden()" "Doc." line=3 vis=pub in=c exported=true sym=hidden
  c::a fn crate_fn "pub(crate) fn crate_fn()" "Doc." line=4 vis=pub(crate) in=c exported=false sym=crate_fn
src/lib.rs
  c fn free "pub fn free()" "Doc." line=2 vis=pub in=c exported=true sym=free
  c trait T "pub trait T" "Doc." line=10 vis=pub in=c exported=true sym=T
  c method S::new "pub fn new()" "Doc." line=12 vis=pub in=c exported=true sym=impl::S::new
src/tests.rs
"#;
