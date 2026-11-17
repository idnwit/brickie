//! The `independent` brick flag: inbound from a sibling needs nothing, outbound only to another
//! independent brick through a rule and a link. Flat root config over the `config::tests` crates.

use crate::config::tests::{cache, edge, fixture, resolved};
use crate::diag::diagnostic;

const BRICKS: &str = r#"
[[brick]]
name = "lib"
members = ["core"]
independent = true
tags = ["lib"]
[[brick]]
name = "other"
members = ["web-api"]
[[brick]]
name = "other-lib"
members = ["web-ui"]
independent = true
"#;

/// Root config: the bricks above (web-shared stays unmapped), then the rules.
fn root(deps: &str) -> String {
    format!("{BRICKS}\n{deps}")
}

fn run(root: &str, edges: &[(&str, &str)]) -> Vec<(String, String)> {
    run_in(root, "", edges)
}

/// Like `run`, with `web` as the nested `crates/web/brickie.toml`.
fn run_in(root: &str, web: &str, edges: &[(&str, &str)]) -> Vec<(String, String)> {
    let (dir, cfg) = fixture(root, web);
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    let mut per = Vec::new();
    for pkg in ["core", "web-api", "web-ui", "web-shared"] {
        let es = edges
            .iter()
            .filter(|(f, _)| *f == pkg)
            .map(|(f, t)| edge(f, t, "prod"))
            .collect();
        per.push((pkg, es, vec![]));
    }
    let c = cache(&ws, &per);
    let mut out: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| {
            let d = diagnostic(f.as_ref());
            (d.code, d.message)
        })
        .filter(|(c, _)| c != "bricks/unmapped")
        .collect();
    out.sort();
    out
}

fn codes(f: &[(String, String)]) -> Vec<&str> {
    f.iter().map(|(c, _)| c.as_str()).collect()
}

fn rule(from: &str, to: &str) -> String {
    format!("[[rule]]\nfrom = \"{from}\"\nto = \"{to}\"\nreason = \"r\"\n")
}

fn link(from: &str, to: &str) -> String {
    format!("[[link]]\nfrom = \"{from}\"\nto = \"{to}\"\nreason = \"r\"\n")
}

#[test]
fn outbound_to_a_normal_brick_is_never_allowed() {
    for rules in [
        String::new(),
        rule("*", "*") + &link("lib", "other"),
        rule("lib", "other") + &link("lib", "other"),
    ] {
        let f = run(&root(&rules), &[("core", "web-api")]);
        assert_eq!(codes(&f), ["deps/unlinked"], "{rules}: {f:#?}");
        assert_eq!(f[0].1, "`lib` (lib) uses `other`");
    }
}

#[test]
fn outbound_to_an_independent_brick_needs_a_rule_and_a_link() {
    // `*` never matches an independent brick.
    let f = run(
        &root(&(rule("*", "*") + &link("lib", "other-lib"))),
        &[("core", "web-ui")],
    );
    assert_eq!(codes(&f), ["deps/unlinked"], "{f:#?}");
    let f = run(&root(&rule("lib", "other-lib")), &[("core", "web-ui")]);
    assert_eq!(
        codes(&f),
        ["deps/unlinked"],
        "a rule alone is not a link: {f:#?}"
    );
    for from in ["lib", "tag:lib"] {
        let f = run(
            &root(&(rule(from, "other-lib") + &link("lib", "other-lib"))),
            &[("core", "web-ui")],
        );
        assert!(f.is_empty(), "{from}: {f:#?}");
    }
}

#[test]
fn edge_inside_an_independent_brick_is_fine() {
    let f = run(&root(""), &[("core", "core")]);
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn inbound_needs_no_rule_and_no_link() {
    let f = run(&root(""), &[("web-api", "core")]);
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn independent_in_a_nested_level_serves_its_siblings() {
    let root = "[[brick]]\nname = \"app\"\nmembers = [\"core\"]\n\
        [[brick]]\nname = \"web\"\npath = \"crates/web\"\n";
    let web = "exports = [\"api\", \"shared\"]\n\
        [[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\n\
        [[brick]]\nname = \"ui\"\nmembers = [\"web-ui\"]\n\
        [[brick]]\nname = \"shared\"\nmembers = [\"web-shared\"]\nindependent = true\n";
    let f = run_in(root, web, &[("web-ui", "web-shared")]);
    assert!(f.is_empty(), "a sibling uses it freely: {f:#?}");
    // From outside, the edge is judged at the root like any other: it needs a link.
    let f = run_in(root, web, &[("core", "web-shared")]);
    assert_eq!(codes(&f), ["deps/unlinked"], "{f:#?}");
    let root = format!("{root}{}{}", rule("app", "web"), link("app", "web/shared"));
    let f = run_in(&root, web, &[("core", "web-shared")]);
    assert!(
        f.is_empty(),
        "an exported lib is reached through a link: {f:#?}"
    );
}

#[test]
fn independent_root_composite_acts_as_one_lib() {
    let root = "[[brick]]\nname = \"app\"\nmembers = [\"core\"]\n\
        [[brick]]\nname = \"web\"\npath = \"crates/web\"\nindependent = true\n";
    let web = format!(
        "exports = [\"api\"]\n\
        [[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\n\
        [[brick]]\nname = \"ui\"\nmembers = [\"web-ui\"]\n\
        [[brick]]\nname = \"shared\"\nmembers = [\"web-shared\"]\n{}{}",
        rule("*", "*"),
        link("ui", "shared")
    );
    let f = run_in(root, &web, &[("web-ui", "web-shared"), ("core", "web-api")]);
    assert!(f.is_empty(), "{f:#?}");
    let f = run_in(root, &web, &[("web-api", "core")]);
    assert_eq!(codes(&f), ["deps/unlinked"], "{f:#?}");
    let f = run_in(root, &web, &[("core", "web-ui")]);
    assert_eq!(
        codes(&f),
        ["deps/private"],
        "exports apply to independents: {f:#?}"
    );
}

#[test]
fn a_link_from_an_independent_to_a_normal_brick_is_denied() {
    let rules = rule("lib", "other") + &link("lib", "other") + &link("other", "lib");
    let (_dir, cfg) = fixture(&root(&rules), "");
    let d = super::design_links(&cfg);
    assert!(
        d[0].denied
            .as_deref()
            .is_some_and(|w| w.contains("independent")),
        "{d:#?}"
    );
    assert!(
        d[1].denied.is_none(),
        "a link into a lib needs no rule: {d:#?}"
    );
}

#[test]
fn a_valid_link_is_planned_until_a_measured_edge_uses_it() {
    let r = root(&(rule("lib", "other-lib") + &link("lib", "other-lib")));
    let planned = |edges: &[(&str, &str)]| {
        let (dir, cfg) = fixture(&r, "");
        let res = resolved(dir.path(), &cfg).unwrap();
        let ws = dir.path().canonicalize().unwrap();
        let es = edges.iter().map(|(f, t)| edge(f, t, "prod")).collect();
        let c = cache(&ws, &[("core", es, vec![])]);
        let marked = super::marked_links(&c, &cfg, &res);
        let only = super::planned_links(&c, &cfg, &res);
        assert_eq!(only.len(), marked.iter().filter(|s| s.planned).count());
        marked.iter().any(|s| s.planned)
    };
    assert!(planned(&[]));
    assert!(!planned(&[("core", "web-ui")]));
}
