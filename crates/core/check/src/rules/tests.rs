//! Rule evaluation on the two-level fixture of `config::tests`: lifting to the common
//! ancestor, `deps/private` with scoped exports, cycles per level and inherited complexity.

use crate::config::tests::{ROOT, WEB, cache, edge, fixture, func, resolved};
use crate::diag::diagnostic;

/// (code, brick_level, message) of every finding.
fn findings(root: &str, web: &str) -> Vec<(String, String, String)> {
    let (dir, cfg) = fixture(root, web);
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(
        &dir.path().canonicalize().unwrap(),
        &[
            (
                "web-ui",
                vec![edge("web-ui", "web-api", "prod")],
                vec![func("ui_fn", "web-ui", 15)],
            ),
            (
                "web-api",
                vec![
                    edge("web-api", "web-ui", "prod"),
                    edge("web-api", "core", "prod"),
                ],
                vec![func("api_fn", "web-api", 15)],
            ),
            (
                "core",
                vec![
                    edge("core", "web-api", "prod"),
                    edge("core", "web-ui", "test"),
                    edge("core", "web-shared", "prod"),
                ],
                vec![func("core_fn", "core", 25)],
            ),
            (
                "web-shared",
                vec![edge("web-shared", "web-api", "prod")],
                vec![],
            ),
        ],
    );
    let mut out: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| {
            let d = diagnostic(f.as_ref());
            (d.code, d.brick_level.unwrap_or_default(), d.message)
        })
        .collect();
    out.sort();
    out
}

fn has(f: &[(String, String, String)], code: &str, level: &str, needle: &str) -> bool {
    f.iter()
        .any(|(c, l, m)| c == code && l == level && m.contains(needle))
}

#[test]
fn edges_are_checked_at_the_common_ancestor() {
    let f = findings(ROOT, WEB);
    // web -> core is allowed at the root; core -> web is not (test edges included).
    assert!(!f.iter().any(|x| x.2.starts_with("`web`")), "{f:#?}");
    assert!(has(
        &f,
        "deps/default",
        "root",
        "`core` (core) uses `web` (app) via `ui` in tests"
    ));
    let root_defaults = f
        .iter()
        .filter(|(c, l, _)| c == "deps/default" && l == "root")
        .count();
    assert_eq!(root_defaults, 3, "{f:#?}");
    // Inside web: shared -> api has no rule.
    assert!(has(&f, "deps/default", "root/web", "`shared` uses `api`"));
    assert_eq!(f.iter().filter(|x| x.0.starts_with("deps/de")).count(), 4);
}

#[test]
fn private_respects_scoped_exports() {
    let f = findings(ROOT, WEB);
    let private: Vec<_> = f.iter().filter(|x| x.0 == "deps/private").collect();
    // core -> shared is hidden; core -> ui (test) passes through the scoped export.
    assert_eq!(private.len(), 1, "{f:#?}");
    assert_eq!(private[0].1, "root/web");
    assert!(private[0].2.contains("`shared`"));
    // Without the scoped entry the test edge into ui is private too.
    let web = WEB.replace(", { brick = \"ui\", scope = \"test\" }", "");
    let f = findings(ROOT, &web);
    assert_eq!(f.iter().filter(|x| x.0 == "deps/private").count(), 2);
}

/// Help texts of the findings with `code` whose message contains `needle`.
fn helps(code: &str, needle: &str) -> Vec<String> {
    let (dir, cfg) = fixture(ROOT, WEB);
    let res = resolved(dir.path(), &cfg).unwrap();
    let c = cache(
        &dir.path().canonicalize().unwrap(),
        &[
            ("web-api", vec![edge("web-api", "web-ui", "prod")], vec![]),
            (
                "core",
                vec![
                    edge("core", "web-ui", "test"),
                    edge("core", "web-shared", "prod"),
                ],
                vec![],
            ),
            (
                "web-shared",
                vec![edge("web-shared", "web-api", "prod")],
                vec![],
            ),
        ],
    );
    super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| diagnostic(f.as_ref()))
        .filter(|d| d.code == code && d.message.contains(needle))
        .filter_map(|d| d.help)
        .collect()
}

#[test]
fn default_help_names_the_scope_for_test_edges() {
    let test = helps("deps/default", "in tests");
    assert_eq!(test.len(), 1, "{test:?}");
    assert!(test[0].contains("brickie design rule add core web allow --scope all"));
    let prod = helps("deps/default", "`shared` uses `api`");
    assert_eq!(prod.len(), 1, "{prod:?}");
    assert!(!prod[0].contains("--scope"));
}

#[test]
fn private_help_gives_the_export_command() {
    let h = helps("deps/private", "`shared`");
    assert_eq!(h.len(), 1, "{h:?}");
    assert!(h[0].contains("brickie design export set shared [--scope prod|test]"));
}

#[test]
fn cycles_are_found_per_level() {
    let f = findings(ROOT, WEB);
    assert!(has(&f, "deps/cycle", "root", "`core`, `web`"), "{f:#?}");
    assert!(has(&f, "deps/cycle", "root/web", "`api`, `ui`"), "{f:#?}");
    let web = WEB.replace("cycles = \"deny\"", "cycles = \"allow\"");
    let f = findings(ROOT, &web);
    assert!(!f.iter().any(|x| x.0 == "deps/cycle" && x.1 == "root/web"));
}

#[test]
fn complexity_rules_of_all_levels_apply_tightest_wins() {
    let f = findings(ROOT, WEB);
    let cx: Vec<_> = f.iter().filter(|x| x.0 == "complexity/cognitive").collect();
    assert_eq!(cx.len(), 2, "{f:#?}");
    assert!(has(
        &f,
        "complexity/cognitive",
        "root/web",
        "`ui_fn` has cognitive 15 (limit 10)"
    ));
    assert!(has(
        &f,
        "complexity/cognitive",
        "root",
        "`core_fn` has cognitive 25 (limit 20)"
    ));
    // A looser nested rule does not relax the root limit.
    let web = WEB.replace("cognitive = 10", "cognitive = 100");
    let f = findings(ROOT, &web);
    assert!(!has(&f, "complexity/cognitive", "root/web", "ui_fn"));
    let root = ROOT.replace("cognitive = 20", "cognitive = 12");
    let f = findings(&root, &web);
    assert!(has(
        &f,
        "complexity/cognitive",
        "root/web",
        "`ui_fn` has cognitive 15 (limit 12)"
    ));
}
