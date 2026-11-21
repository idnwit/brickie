//! `Item::surface`: pub fields, variants, trait members and implemented traits.

use crate::run::complexity::Analysis;
use crate::run::items::add_items;
use crate::scip::testutil::Builder;
use brickie_facts::FileData;
use std::collections::{BTreeMap, HashSet};

#[test]
fn surface_of_types_and_traits() {
    let mut b = Builder::default();
    let lib = b.file("src/lib.rs");
    let root = b.module(lib, "c", "", None);
    let rows = [
        ("V", "V#", "pub struct V"),
        ("rank", "V#rank.", "pub rank: Rank"),
        ("hid", "V#hid.", "hid: u8"),
        ("E", "E#", "pub enum E"),
        ("A", "E#A#", "A(Rank)"),
        ("x", "E#B#x.", "pub x: Elem"),
        ("Tr", "Tr#", "pub trait Tr"),
        ("m", "Tr#m().", "fn m(&self) -> Out"),
        ("P", "P#", "pub struct P"),
    ];
    for (i, (name, tail, sig)) in rows.into_iter().enumerate() {
        let s = b.item(lib, name, root);
        b.g.symbols[s].scip = format!("rust-analyzer cargo p 0.1.0 {tail}");
        b.g.symbols[s].signature = sig.into();
        b.g.symbols[s].module = Some(root);
        b.g.symbols[s].range = (i as u32, 0, i as u32, 1);
    }
    let an = Analysis {
        impls: vec![("V".into(), "Tr".into()), ("V".into(), "Debug".into())],
        ..Default::default()
    };
    let analyses: BTreeMap<String, Analysis> = [("src/lib.rs".to_string(), an)].into();
    let mut files: BTreeMap<String, FileData> = BTreeMap::new();
    files.insert(
        "src/lib.rs".into(),
        crate::run::e2e_tests::items::file_data(),
    );
    add_items(
        &mut files,
        &b.g,
        &b.ids,
        &["c".to_string()],
        &analyses,
        &HashSet::new(),
    );
    let surf = |n: &str| -> Vec<String> {
        files["src/lib.rs"]
            .items
            .iter()
            .find(|i| i.name == n)
            .unwrap_or_else(|| panic!("no item {n}"))
            .surface
            .clone()
    };
    // Private fields are not surface; implemented traits are.
    assert_eq!(surf("V"), ["Debug", "Tr", "pub rank: Rank"]);
    // Variants and their pub fields.
    assert_eq!(surf("E"), ["A(Rank)", "pub x: Elem"]);
    assert_eq!(surf("Tr"), ["fn m(&self) -> Out"]);
    assert!(surf("P").is_empty());
}
