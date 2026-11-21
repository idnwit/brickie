//! `Item::allow`: the directive comment above an item, matched by the line of the code below it.

use crate::run::complexity::Analysis;
use crate::run::items::add_items;
use crate::scip::testutil::Builder;
use brickie_facts::FileData;
use brickie_metrics::Directive;
use std::collections::{BTreeMap, HashSet};

#[test]
fn directive_is_attached_to_the_item_below_it() {
    let mut b = Builder::default();
    let lib = b.file("src/lib.rs");
    let root = b.module(lib, "c", "", None);
    for (i, name) in ["a", "b"].into_iter().enumerate() {
        let s = b.item(lib, name, root);
        b.g.symbols[s].scip = format!("rust-analyzer cargo p 0.1.0 {name}#");
        b.g.symbols[s].signature = format!("pub struct {name}");
        b.g.symbols[s].module = Some(root);
        // `a` on line 4, `b` on line 8 (1-based).
        b.g.symbols[s].range = (3 + 4 * i as u32, 0, 3 + 4 * i as u32, 1);
    }
    let an = Analysis {
        directives: vec![Directive {
            line: 2,
            target: Some(4),
            codes: vec!["interface/unused".into()],
            reason: "why".into(),
        }],
        ..Default::default()
    };
    let analyses: BTreeMap<String, Analysis> = [("src/lib.rs".to_string(), an)].into();
    let mut files: BTreeMap<String, FileData> = BTreeMap::new();
    files.insert(
        "src/lib.rs".into(),
        crate::run::e2e_tests::items::file_data(),
    );
    let ids = ["c".to_string()];
    add_items(&mut files, &b.g, &b.ids, &ids, &analyses, &HashSet::new());
    let allow = |n: &str| {
        let it = files["src/lib.rs"]
            .items
            .iter()
            .find(|i| i.name == n)
            .unwrap();
        it.allow.clone()
    };
    let a = allow("a").unwrap();
    assert_eq!(
        (a.line, a.codes, a.reason),
        (2, vec!["interface/unused".to_string()], "why".to_string())
    );
    assert!(allow("b").is_none());
}
