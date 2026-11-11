use super::testcfg::is_test_cfg;
use super::*;

fn one(src: &str) -> FnMetrics {
    analyze(src).fns.remove(0)
}

#[test]
fn cfg_detection() {
    assert!(is_test_cfg("#[cfg(test)]"));
    assert!(is_test_cfg("#[cfg( any( test , feature = \"x\" ) )]"));
    assert!(is_test_cfg("#[cfg(all(test, unix))]"));
    assert!(is_test_cfg("#[cfg(feature = \"test-util\")]"));
    assert!(is_test_cfg("#![cfg(test)]"));
    assert!(!is_test_cfg("#[cfg(not(test))]"));
    assert!(!is_test_cfg("#[cfg(any(unix, not(test)))]"));
    assert!(!is_test_cfg("#[cfg(feature = \"test\")]"));
    assert!(!is_test_cfg("#[cfg(feature = \"testing\")]"));
    assert!(!is_test_cfg("#[derive(Debug)]"));
}

#[test]
fn straight_line() {
    let m = one("fn f(a: u32, b: u32) -> u32 { a + b }");
    assert_eq!(
        (m.cognitive, m.cyclomatic, m.nesting, m.paths, m.params),
        (0, 1, 0, 1, 2)
    );
}

#[test]
fn params_do_not_count_self() {
    let a = analyze(
        "struct S;\nimpl S { fn f(&self, a: u32) {} fn g(self) {} fn h(&mut self, a: u8, b: u8) {} }",
    );
    let p: Vec<u32> = a.fns.iter().map(|f| f.params).collect();
    assert_eq!(p, vec![1, 0, 2]);
}

#[test]
fn if_else_chain() {
    let m = one("fn f(x: i32) -> i32 { if x > 0 { 1 } else if x < 0 { -1 } else { 0 } }");
    // cognitive: if +1, else if +1, else +1; cyclomatic 1+2; paths 1 + (1 + 1).
    assert_eq!((m.cognitive, m.cyclomatic, m.paths), (3, 3, 3));
}

#[test]
fn nesting_and_logic() {
    let m = one("fn f(a: bool, b: bool, c: bool) { for i in 0..3 { if a && b || c { g(i); } } }");
    // for +1; if +2 (nesting 1); `&&` then `||` = 2 sequences; total 5.
    assert_eq!((m.cognitive, m.nesting), (5, 2));
    assert_eq!(m.cyclomatic, 1 + 1 + 1 + 2);
    // if = B(then) 1 + no-else 1 + 2 logic operators = 4; for = B(if) + 1.
    assert_eq!(m.paths, 5);
}

#[test]
fn fn_tree_of_the_enclosing_function() {
    let src = "fn a() {}\nfn b(x: i32) -> i32 {\n    if x > 0 && x < 9 {\n        for i in 0..x { g(i); }\n    }\n    0\n}\n";
    let t = fn_tree(src, 4).unwrap();
    assert_eq!((t.label.as_str(), t.line, t.end_line), ("b", 2, 7));
    let kinds: Vec<(&str, u32)> = t.nodes.iter().map(|n| (n.kind.as_str(), n.depth)).collect();
    assert_eq!(kinds, vec![("if", 1), ("cond_op", 2), ("loop", 2)]);
    assert!(fn_tree(src, 100).is_none());
}

#[test]
fn test_gating() {
    let a = analyze(
        "fn p() {}\n#[cfg(test)]\nmod t {\n    #[test]\n    fn q() {}\n}\n#[cfg(not(test))]\nfn r() {}\n",
    );
    let t: Vec<(&str, bool)> = a.fns.iter().map(|f| (f.name.as_str(), f.test)).collect();
    assert_eq!(t, vec![("p", false), ("q", true), ("r", false)]);
    assert!(a.is_test_at((4, 4)));
    assert!(!a.is_test_at((0, 0)));
}
