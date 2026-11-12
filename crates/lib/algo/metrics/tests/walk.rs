//! Characterization table for `cognitive::walk`: source -> (cognitive, cyclomatic, max depth).
//! Expected values are what the implementation returns today, including the odd ones.

use brickie_metrics::analyze;

const CASES: &[(&str, &str, (u32, u32, u32))] = &[
    ("empty", "fn f() {}", (0, 1, 0)),
    ("if", "fn f(a: bool) { if a { g(); } }", (1, 2, 1)),
    (
        "if else",
        "fn f(a: bool) { if a { g(); } else { h(); } }",
        (2, 2, 1),
    ),
    (
        "if else-if else",
        "fn f(x: i32) -> i32 { if x > 0 { 1 } else if x < 0 { -1 } else { 0 } }",
        (3, 3, 1),
    ),
    (
        "nested if in if and else",
        "fn f(a: bool, b: bool) { if a { if b { g(); } } else { if b { h(); } } }",
        (6, 4, 2),
    ),
    (
        "match one arm",
        "fn f(x: u8) { match x { _ => g() } }",
        (1, 1, 1),
    ),
    (
        "match many arms",
        "fn f(x: u8) { match x { 0 => a(), 1 => b(), 2 => c(), _ => d() } }",
        (1, 4, 1),
    ),
    (
        "match in if",
        "fn f(a: bool, x: u8) { if a { match x { 0 => a(), _ => b() } } }",
        (3, 3, 2),
    ),
    (
        "for while loop",
        "fn f() { for i in 0..3 { g(i); } while c() { g(0); } loop { break; } }",
        (3, 4, 1),
    ),
    (
        "nested loops",
        "fn f() { for i in 0..3 { while c() { loop { break; } } } }",
        (6, 4, 3),
    ),
    (
        "let else",
        "fn f(o: Option<u8>) { let Some(x) = o else { return; }; g(x); }",
        (1, 2, 1),
    ),
    (
        "let else nested in if",
        "fn f(a: bool, o: Option<u8>) { if a { let Some(x) = o else { return; }; g(x); } }",
        (3, 3, 2),
    ),
    (
        "labeled break continue",
        "fn f() { 'o: for i in 0..3 { for j in 0..3 { if j > i { continue 'o; } if i > 5 { break 'o; } } } }",
        (11, 5, 3),
    ),
    (
        "unlabeled break continue",
        "fn f() { for i in 0..3 { if i > 1 { continue; } break; } }",
        (3, 3, 2),
    ),
    (
        "recursion plain",
        "fn f(n: u32) { if n > 0 { f(n - 1); } }",
        (2, 2, 1),
    ),
    (
        "recursion self method",
        "struct S;\nimpl S { fn f(&self, n: u32) { if n > 0 { self.f(n - 1); } } }",
        (2, 2, 1),
    ),
    (
        "recursion Self path",
        "struct S;\nimpl S { fn f(n: u32) { if n > 0 { Self::f(n - 1); } } }",
        (2, 2, 1),
    ),
    (
        "not recursion other receiver and path",
        "struct S;\nimpl S { fn f(&self, o: &S) { o.f(); S::f(o); g::f(); } }",
        (0, 1, 0),
    ),
    (
        "closure",
        "fn f(v: Vec<u8>) { v.iter().for_each(|x| g(*x)); }",
        (0, 1, 1),
    ),
    (
        "nested closures with branches",
        "fn f(a: bool) { let c = |x: u8| { let d = |y: u8| if a { y } else { x }; d(1) }; c(2); }",
        (4, 2, 3),
    ),
    (
        "logic single chain",
        "fn f(a: bool, b: bool, c: bool) -> bool { a && b && c }",
        (1, 3, 0),
    ),
    (
        "logic mixed chain",
        "fn f(a: bool, b: bool, c: bool) -> bool { a && b || c }",
        (2, 3, 0),
    ),
    (
        "logic alternating with parens",
        "fn f(a: bool, b: bool, c: bool, d: bool) -> bool { (a || b) && (c || d) }",
        (3, 4, 0),
    ),
    (
        "logic in if condition",
        "fn f(a: bool, b: bool, c: bool) { if a && b || c { g(); } }",
        (3, 4, 1),
    ),
    (
        "logic inside nested call arguments",
        "fn f(a: bool, b: bool) { g(a && b, h(a || b)); }",
        (2, 3, 0),
    ),
    (
        "question mark",
        "fn f() -> Option<u8> { let a = g()?; let b = h()?.k()?; Some(a + b) }",
        (0, 4, 0),
    ),
    (
        "macro body",
        "fn f(a: bool) { println!(\"{}\", if a { 1 } else { 2 }); vec![a && a]; }",
        (0, 1, 0),
    ),
    (
        "nested fn is skipped",
        "fn f(a: bool) { fn g(b: bool) { if b { h(); } } if a { g(a); } }",
        (1, 2, 1),
    ),
    (
        "test mod inside a body is skipped",
        "fn f(a: bool) { #[cfg(test)] mod t { fn x(b: bool) { if b {} } } if a {} }",
        (1, 2, 1),
    ),
];

#[test]
fn walk_table() {
    let mut bad = Vec::new();
    for (name, src, want) in CASES {
        let a = analyze(src);
        let m = a.fns.iter().find(|f| f.name == "f").unwrap();
        let got = (m.cognitive, m.cyclomatic, m.nesting);
        if got != *want {
            bad.push(format!("{name}: {got:?}"));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}
