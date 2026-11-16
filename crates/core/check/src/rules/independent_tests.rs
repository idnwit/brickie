//! The `independent` brick flag: outbound errors, inbound deny handling, load-time errors and
//! the unused-link exclusion. Flat root config over the `config::tests` crates.

use crate::config::tests::{cache, edge, fixture, resolved};
use crate::config::{Config, load};
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

/// Root config: the bricks above (web-shared stays unmapped), then `deps`.
fn root(deps: &str) -> String {
    format!("{BRICKS}\n{deps}")
}

fn run(root: &str, edges: &[(&str, &str)]) -> Vec<(String, String)> {
    let (dir, cfg) = fixture(root, "");
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

const ALLOW_ALL: &str = "[deps]\ndefault = \"allow\"\n";
const DENY_ALL: &str = "[deps]\ndefault = \"deny\"\n";

#[test]
fn outbound_edge_to_a_brick_is_independent_error_only() {
    for deps in [ALLOW_ALL, DENY_ALL] {
        let f = run(&root(deps), &[("core", "web-api")]);
        assert_eq!(codes(&f), ["deps/independent"], "{f:#?}");
        assert_eq!(f[0].1, "`lib` is independent and uses `other`");
    }
}

#[test]
fn outbound_edge_to_another_independent_brick_is_an_error() {
    let f = run(&root(ALLOW_ALL), &[("core", "web-ui")]);
    assert_eq!(codes(&f), ["deps/independent"], "{f:#?}");
    assert!(f[0].1.contains("`other-lib`"));
}

#[test]
fn outbound_edge_to_an_unmapped_module_is_an_error() {
    let f = run(&root(ALLOW_ALL), &[("core", "web-shared")]);
    assert_eq!(codes(&f), ["deps/independent"], "{f:#?}");
    assert!(f[0].1.contains("`web-shared::m`"), "{f:#?}");
}

#[test]
fn outbound_error_ignores_allow_rules_private_and_pick_rule() {
    let deps =
        "[deps]\ndefault = \"deny\"\n[[deps.rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n";
    let f = run(&root(deps), &[("core", "web-api")]);
    assert_eq!(codes(&f), ["deps/independent"], "{f:#?}");
}

#[test]
fn edge_inside_an_independent_brick_is_fine() {
    let f = run(&root(DENY_ALL), &[("core", "core")]);
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn inbound_edge_is_allowed_by_default_even_under_default_deny() {
    let f = run(&root(DENY_ALL), &[("web-api", "core")]);
    assert!(f.is_empty(), "{f:#?}");
}

#[test]
fn inbound_deny_by_brick_or_tag_is_a_violation() {
    for to in ["lib", "tag:lib"] {
        let deps = format!(
            "{ALLOW_ALL}[[deps.rule]]\nfrom = \"other\"\nto = \"{to}\"\naction = \"deny\"\n"
        );
        let f = run(&root(&deps), &[("web-api", "core")]);
        assert_eq!(codes(&f), ["deps/deny"], "{to}: {f:#?}");
    }
}

#[test]
fn inbound_star_deny_never_matches_an_independent_target() {
    let deps =
        format!("{ALLOW_ALL}[[deps.rule]]\nfrom = \"other\"\nto = \"*\"\naction = \"deny\"\n");
    let f = run(&root(&deps), &[("web-api", "core")]);
    assert!(f.is_empty(), "{f:#?}");
    let f = run(&root(&deps), &[("web-api", "web-shared")]);
    assert!(f.is_empty(), "unmapped is ignored: {f:#?}");
}

#[test]
fn inbound_more_specific_allow_beats_a_deny() {
    let deps = format!(
        "{ALLOW_ALL}[[deps.rule]]\nfrom = \"*\"\nto = \"tag:lib\"\naction = \"deny\"\n\
         [[deps.rule]]\nfrom = \"other\"\nto = \"lib\"\naction = \"allow\"\n"
    );
    let f = run(&root(&deps), &[("web-api", "core")]);
    assert!(f.is_empty(), "{f:#?}");
}

fn load_err(root: &str, web: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("crates/web")).unwrap();
    std::fs::write(dir.path().join("brickie.toml"), root).unwrap();
    std::fs::write(dir.path().join("crates/web/brickie.toml"), web).unwrap();
    format!("{:#}", load(&dir.path().join("brickie.toml")).unwrap_err())
}

#[test]
fn independent_in_a_nested_level_is_a_config_error() {
    let root = "[[brick]]\nname = \"web\"\npath = \"crates/web\"\n[deps]\ndefault = \"allow\"\n";
    let web = "[[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\nindependent = true\n";
    assert!(load_err(root, web).contains("config/independent-nested"));
}

#[test]
fn independent_composite_is_a_config_error() {
    let root = "[[brick]]\nname = \"web\"\npath = \"crates/web\"\nindependent = true\n";
    let web = "[[brick]]\nname = \"api\"\nmembers = [\"web-api\"]\n";
    assert!(load_err(root, web).contains("config/independent-nested"));
}

#[test]
fn allow_rule_from_an_independent_brick_is_a_config_error() {
    let deps = "[deps]\ndefault = \"deny\"\n[[deps.rule]]\nfrom = \"lib\"\nto = \"other\"\naction = \"allow\"\n";
    let e = load_err(&root(deps), "");
    assert!(e.contains("config/independent-rule"), "{e}");
}

#[test]
fn deny_rule_from_an_independent_brick_loads() {
    let deps = "[deps]\ndefault = \"deny\"\n[[deps.rule]]\nfrom = \"lib\"\nto = \"other\"\naction = \"deny\"\n";
    let (_d, cfg): (_, Config) = fixture(&root(deps), "");
    assert!(cfg.brick("lib").unwrap().independent);
    assert!(!cfg.brick("other").unwrap().independent);
}

#[test]
fn allow_rule_to_an_independent_brick_is_never_unused() {
    let deps = "[deps]\ndefault = \"deny\"\n\
        [[deps.rule]]\nfrom = \"other\"\nto = \"lib\"\naction = \"allow\"\n\
        [[deps.rule]]\nfrom = \"other\"\nto = \"other-lib\"\naction = \"allow\"\n";
    let (dir, cfg) = fixture(&root(deps), "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(&dir.path().canonicalize().unwrap(), &[]);
    assert!(crate::rules::unused_links(&c, &cfg, &res).is_empty());
}

#[test]
fn allow_rule_to_a_normal_brick_is_still_unused_without_edges() {
    let root = "[[brick]]\nname = \"a\"\nmembers = [\"core\"]\n[[brick]]\nname = \"b\"\nmembers = [\"web-api\"]\n\
        [deps]\ndefault = \"deny\"\n[[deps.rule]]\nfrom = \"a\"\nto = \"b\"\naction = \"allow\"\n";
    let (dir, cfg) = fixture(root, "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(&dir.path().canonicalize().unwrap(), &[]);
    assert_eq!(crate::rules::unused_links(&c, &cfg, &res).len(), 1);
}

fn allow(from: &str, to: &str) -> String {
    format!("[[deps.rule]]\nfrom = \"{from}\"\nto = \"{to}\"\naction = \"allow\"\n")
}

#[test]
fn independent_may_use_independent_with_an_explicit_allow_rule() {
    for (a, b) in [("lib", "other-lib"), ("tag:lib", "other-lib")] {
        let deps = format!("{DENY_ALL}{}", allow(a, b));
        let f = run(&root(&deps), &[("core", "web-ui")]);
        assert!(f.is_empty(), "{a}->{b}: {f:#?}");
    }
}

#[test]
fn star_allow_does_not_permit_independent_to_independent() {
    for (a, b) in [("*", "other-lib"), ("lib", "*"), ("*", "*")] {
        let deps = format!("{DENY_ALL}{}", allow(a, b));
        let f = run(&root(&deps), &[("core", "web-ui")]);
        assert_eq!(codes(&f), ["deps/independent"], "{a}->{b}: {f:#?}");
    }
}

#[test]
fn independent_to_a_normal_brick_is_an_error_whatever_the_rules() {
    // A rule from the independent brick by name is `config/independent-rule`; `*` loads.
    let deps = format!("{ALLOW_ALL}{}", allow("*", "other"));
    let f = run(&root(&deps), &[("core", "web-api")]);
    assert_eq!(codes(&f), ["deps/independent"], "{f:#?}");
}

#[test]
fn allow_between_independents_loads_and_unbacked_one_is_unused() {
    let deps = format!("{DENY_ALL}{}", allow("lib", "other-lib"));
    let (dir, cfg) = fixture(&root(&deps), "");
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(&dir.path().canonicalize().unwrap(), &[]);
    assert_eq!(crate::rules::unused_links(&c, &cfg, &res).len(), 1);
    let edges = vec![edge("core", "web-ui", "prod")];
    let c = cache(
        &dir.path().canonicalize().unwrap(),
        &[("core", edges, vec![])],
    );
    assert!(crate::rules::unused_links(&c, &cfg, &res).is_empty());
}

#[test]
fn cycles_among_independent_bricks_are_reported() {
    let deps = format!(
        "[deps]\ndefault = \"deny\"\ncycles = \"deny\"\n{}{}",
        allow("lib", "other-lib"),
        allow("other-lib", "lib")
    );
    let f = run(&root(&deps), &[("core", "web-ui"), ("web-ui", "core")]);
    assert_eq!(codes(&f), ["deps/cycle"], "{f:#?}");
}
