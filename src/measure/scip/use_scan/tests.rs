use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Usage {
    Direct,
    Import,
}

/// `Import` if the position lies inside any use item (sorted and disjoint), else `Direct`.
fn classify(uses: &[UseItem], line: u32, col: u32) -> Usage {
    let idx = uses.partition_point(|u| u.start_pos <= (line, col));
    match idx.checked_sub(1).map(|i| &uses[i]) {
        Some(u) if u.contains(line, col) => Usage::Import,
        _ => Usage::Direct,
    }
}

fn texts(src: &str) -> Vec<&str> {
    use_items(src)
        .iter()
        .map(|u| &src[u.start..u.end])
        .collect()
}

#[test]
fn step1_check6_plain_use() {
    let src = "use a::b;\nfn f() { b(); }\n";
    let u = use_items(src);
    assert_eq!(u.len(), 1);
    assert_eq!((u[0].start_pos, u[0].end_pos), ((0, 0), (0, 9)));
    assert!(!u[0].is_pub);
    assert_eq!(u[0].vis, "");
    assert_eq!(classify(&u, 0, 8), Usage::Import);
    assert_eq!(classify(&u, 0, 9), Usage::Direct);
    assert_eq!(classify(&u, 1, 11), Usage::Direct);
}

#[test]
fn step1_check6_visibilities() {
    let src = "pub use a::A;\npub(crate) use b::B;\npub(in crate::x) use c::C;\npub ( super ) use d::D;\n";
    let u = use_items(src);
    let vis: Vec<_> = u.iter().map(|u| u.vis.as_str()).collect();
    assert_eq!(vis, ["pub", "pub(crate)", "pub(in crate::x)", "pub(super)"]);
    assert!(u.iter().all(|u| u.is_pub));
    assert_eq!(u[1].start_pos, (1, 0));
}

#[test]
fn step1_check6_multiline_tree() {
    let src = "use a::{\n    b,\n    c::{d, e},\n};\nfn x() {}\n";
    let u = use_items(src);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0].end_pos, (3, 2));
    assert_eq!(classify(&u, 2, 8), Usage::Import);
    assert_eq!(classify(&u, 4, 0), Usage::Direct);
}

#[test]
fn step1_check6_inside_fn_and_mod() {
    let src = "fn f() {\n    use x::y;\n    let _ = 1;\n}\nmod m {\n    pub use z::Z;\n    mod n { use q::Q; }\n}\n";
    assert_eq!(texts(src), ["use x::y;", "pub use z::Z;", "use q::Q;"]);
}

#[test]
fn step1_check6_not_in_strings_or_comments() {
    let src = r####"
// use a::b;
/* use c::d; /* nested use e::f; */ use g::h; */
const S: &str = "use i::j;";
const B: &[u8] = b"use k::l;";
const R: &str = r#"use m::n; " still"#;
const R2: &str = r###"use o::p; "## use q::r;"###;
/// use doc::comment;
fn f() {}
"####;
    assert!(use_items(src).is_empty(), "{:?}", texts(src));
}

#[test]
fn step1_check6_comment_between_tokens() {
    let src = "use /* c */ a::b; // tail\nuse c::d;";
    assert_eq!(texts(src), ["use /* c */ a::b;", "use c::d;"]);
}

#[test]
fn step1_check6_lifetime_next_to_char_literal() {
    let src = "fn f<'a>(x: &'a str) -> char { let c = 'x'; let q = '\\''; let n = '\\n'; let u = '\u{e9}'; 'static; c }\nuse a::b;\nconst L: &'static str = \"s\";\nuse c::d;";
    assert_eq!(texts(src), ["use a::b;", "use c::d;"]);
    // `'use` as a label is not a keyword
    assert!(use_items("fn f() { 'use: loop { break 'use; } }").is_empty());
    // a quote char literal must not swallow following code
    assert_eq!(texts("const Q: char = '\"';\nuse a::b;"), ["use a::b;"]);
}

#[test]
fn step1_check6_attributes_before_use() {
    let src = "#[allow(unused_imports)]\nuse a::b;\n#[cfg(test)] #[doc = \"x]\"]\npub use c::d;\n#![allow(x)]\nuse e::f;";
    let u = use_items(src);
    assert_eq!(texts(src), ["use a::b;", "pub use c::d;", "use e::f;"]);
    assert_eq!(u[0].start_pos, (1, 0));
}

#[test]
fn step1_check6_raw_identifiers_and_lookalikes() {
    let src = "fn f() { let r#use = 1; reuse(); user::x(); }\nstruct S { r#use: u8, reuse: u8 }\nfn g() { s.r#use; }\nuse_it();\n";
    assert!(use_items(src).is_empty(), "{:?}", texts(src));
    // not at item start: `x use`-ish garbage and field access
    assert!(use_items("let a = b use;").is_empty());
}

#[test]
fn step1_check6_macro_rules_body_skipped() {
    let src = "macro_rules! m {\n    () => { use a::b; };\n}\nuse c::d;\nmacro_rules! n ( () => { use e::f; } );\nuse g::h;";
    assert_eq!(texts(src), ["use c::d;", "use g::h;"]);
}

#[test]
fn step1_check6_utf8_columns_are_bytes() {
    let src = "const S: &str = \"é\"; use a::b;\n";
    let u = use_items(src);
    // `é` is 2 bytes: "const S: &str = \"é\"; " is 22 bytes
    assert_eq!(u[0].start_pos, (0, 22));
    assert_eq!(u[0].end_pos, (0, 31));
}

#[test]
fn step1_check6_classify_empty_and_crlf() {
    assert_eq!(classify(&[], 0, 0), Usage::Direct);
    let u = use_items("use a;\r\nuse b;\r\nfn f() {}");
    assert_eq!(u.len(), 2);
    assert_eq!(u[1].start_pos, (1, 0));
    assert_eq!(classify(&u, 1, 4), Usage::Import);
}
