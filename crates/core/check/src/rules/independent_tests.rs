//! The `independent` brick flag through `decide`: inbound allowed unless a deny names it,
//! outbound only to another independent brick under an allow naming both. Flat root config over
//! the `config::tests` crates.

use crate::config::load;
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

fn rule(from: &str, to: &str, action: &str) -> String {
    format!("[[rule]]\nfrom = \"{from}\"\nto = \"{to}\"\naction = \"{action}\"\n")
}

#[test]
fn outbound_to_a_normal_brick_is_denied_whatever_the_rules() {
    for rules in [
        String::new(),
        rule("*", "*", "allow"),
        rule("lib", "other", "allow"),
    ] {
        let f = run(&root(&rules), &[("core", "web-api")]);
        assert_eq!(codes(&f), ["deps/deny"], "{rules}: {f:#?}");
        assert_eq!(f[0].1, "`lib` (lib) uses `other`");
    }
}

#[test]
fn outbound_to_an_independent_brick_needs_an_allow_naming_both() {
    let f = run(&root(&rule("*", "*", "allow")), &[("core", "web-ui")]);
    assert_eq!(codes(&f), ["deps/deny"], "{f:#?}");
    for from in ["lib", "tag:lib"] {
        let f = run(
            &root(&rule(from, "other-lib", "allow")),
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
fn inbound_is_allowed_unless_a_deny_names_it() {
    let f = run(&root(""), &[("web-api", "core")]);
    assert!(f.is_empty(), "{f:#?}");
    let f = run(&root(&rule("*", "*", "deny")), &[("web-api", "core")]);
    assert!(
        f.is_empty(),
        "`*` never matches an independent brick: {f:#?}"
    );
    for to in ["lib", "tag:lib"] {
        let f = run(&root(&rule("other", to, "deny")), &[("web-api", "core")]);
        assert_eq!(codes(&f), ["deps/deny"], "{to}: {f:#?}");
    }
    let rules = rule("*", "lib", "deny") + &rule("other", "lib", "allow");
    let f = run(&root(&rules), &[("web-api", "core")]);
    assert!(f.is_empty(), "the more specific allow wins: {f:#?}");
}

#[test]
fn independent_in_a_nested_level_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("crates/web")).unwrap();
    std::fs::write(
        dir.path().join("brickie.toml"),
        "[[brick]]\nname = \"web\"\npath = \"crates/web\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("crates/web/brickie.toml"),
        "[[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\nindependent = true\n",
    )
    .unwrap();
    let e = format!("{:#}", load(&dir.path().join("brickie.toml")).unwrap_err());
    assert!(e.contains("config/invalid"), "{e}");
}

#[test]
fn independent_root_composite_acts_as_one_lib() {
    let root = "[[brick]]\nname = \"app\"\nmembers = [\"core\"]\n\
        [[brick]]\nname = \"web\"\npath = \"crates/web\"\nindependent = true\n";
    let web = "exports = [\"api\"]\n\
        [[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\n\
        [[brick]]\nname = \"ui\"\nmembers = [\"web-ui\"]\n\
        [[brick]]\nname = \"shared\"\nmembers = [\"web-shared\"]\n\
        [[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n";
    let f = run_in(root, web, &[("web-ui", "web-shared"), ("core", "web-api")]);
    assert!(f.is_empty(), "{f:#?}");
    let f = run_in(root, web, &[("web-api", "core")]);
    assert_eq!(codes(&f), ["deps/deny"], "{f:#?}");
    let f = run_in(root, web, &[("core", "web-ui")]);
    assert_eq!(
        codes(&f),
        ["deps/private"],
        "exports apply to independents: {f:#?}"
    );
}

#[test]
fn unused_links_skip_allows_that_never_apply() {
    let rules = rule("lib", "other", "allow") + &rule("other", "lib", "allow");
    let (dir, cfg) = fixture(&root(&rules), "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(&dir.path().canonicalize().unwrap(), &[]);
    assert!(super::unused_links(&c, &cfg, &res).is_empty());
    let rules = rule("lib", "other-lib", "allow");
    let (dir, cfg) = fixture(&root(&rules), "");
    let res = resolved(dir.path(), &cfg).unwrap();
    assert_eq!(super::unused_links(&c, &cfg, &res).len(), 1);
}
