//! Check tests: cycles and complexity rules.

use super::support::*;

const OPEN: &str = "[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n";

#[test]
fn cycles() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks(OPEN));
    assert!(codes(&o, "deps/cycle").is_empty());
    assert_eq!(o.code, 1, "the unmapped module is an error");

    let w = world(|c| {
        c["files"]["crates/port/src/t.rs"]["edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("port::t", "eng::x", "uses", "prod", 3));
    });
    let o = check(&w, &with_bricks(OPEN));
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps/cycle");
    assert_eq!(d.len(), 1);
    assert!(
        d[0]["message"]
            .as_str()
            .unwrap()
            .contains("`adapter`, `engine`, `port`")
    );
    assert_eq!(
        d[0]["notes"][0]["text"],
        "cycle: adapter -> port -> engine -> adapter"
    );
    // One structured edge per brick edge inside the cycle, each with evidence and a note.
    let mut hops: Vec<(&str, &str)> = d[0]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["from"].as_str().unwrap(), e["to"].as_str().unwrap()))
        .collect();
    hops.sort();
    assert_eq!(
        hops,
        [
            ("adapter", "port"),
            ("engine", "adapter"),
            ("engine", "port"),
            ("port", "engine")
        ]
    );
    assert!(
        d[0]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["span"]["line"].as_u64().unwrap() > 0)
    );
    assert_eq!(d[0]["notes"].as_array().unwrap().len(), 5);
    assert!(d[0]["span"]["file"].is_string());
    let text = run(&w.save(&with_bricks(OPEN)), &[]).stdout;
    assert!(
        text.contains("`port` -> `engine` is on a cycle, used at"),
        "{text}"
    );
    // Sibling cycles are errors even when no rule is written.
    let o = check(&w, &with_bricks(""));
    assert_eq!(codes(&o, "deps/cycle").len(), 1);
}

// ---- complexity ----

#[test]
fn complexity_error_and_pass() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks("[[complexity]]\nbricks = \"tag:core\"\ncognitive = 15\n"),
    );
    assert_eq!(o.code, 1);
    let d = codes(&o, "complexity");
    assert_eq!(d.len(), 1, "test-scope and other bricks are not checked");
    assert_eq!(d[0]["code"], "complexity/cognitive");
    assert_eq!(d[0]["message"], "`busy` has cognitive 20 (limit 15)");
    assert_eq!(d[0]["span"]["line"], 5);
    // At the limit passes.
    let o = check(
        &w,
        &with_bricks("[[complexity]]\nbricks = \"tag:core\"\ncognitive = 20\n"),
    );
    assert!(codes(&o, "complexity").is_empty());
}

#[test]
fn complexity_scope_and_specificity() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks("[[complexity]]\nbricks = \"*\"\ncognitive = 15\n"),
    );
    assert_eq!(codes(&o, "complexity").len(), 2); // busy and heavy
    // scope = "all" includes test functions.
    let o = check(
        &w,
        &with_bricks("[[complexity]]\nscope = \"all\"\ncognitive = 15\n"),
    );
    assert_eq!(codes(&o, "complexity").len(), 3);
    // The brick rule overrides the tag rule for the metric it sets.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[complexity]]
bricks = "tag:core"
cognitive = 15
[[complexity]]
bricks = "engine"
cognitive = 25
"#,
        ),
    );
    assert!(codes(&o, "complexity").is_empty());
    // Other metrics: params.
    let o = check(&w, &with_bricks("[[complexity]]\nparams = 1\n"));
    assert!(!codes(&o, "complexity/params").is_empty());
}
