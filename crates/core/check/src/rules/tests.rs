//! Rule evaluation on the two-level fixture of `config::tests`: lifting to the common
//! ancestor, `deps/private`, cycles per level and complexity per level.

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
    // web -> core is allowed at the root; core -> web matches no rule (test edges included).
    assert!(!f.iter().any(|x| x.2.starts_with("`web`")), "{f:#?}");
    assert!(has(
        &f,
        "deps/deny",
        "root",
        "`core` (core) uses `web` (app) via `ui` in tests"
    ));
    let root = f
        .iter()
        .filter(|(c, l, _)| c == "deps/deny" && l == "root")
        .count();
    assert_eq!(root, 3, "{f:#?}");
    // Inside web: shared -> api has no rule.
    assert!(has(&f, "deps/deny", "root/web", "`shared` uses `api`"));
    assert_eq!(f.iter().filter(|x| x.0 == "deps/deny").count(), 4);
}

#[test]
fn private_hides_unexported_children() {
    let f = findings(ROOT, WEB);
    let private: Vec<_> = f.iter().filter(|x| x.0 == "deps/private").collect();
    // core -> shared and core -> ui (test) enter web through children it does not export.
    assert_eq!(private.len(), 2, "{f:#?}");
    assert!(private.iter().all(|p| p.1 == "root/web"));
    let web = WEB.replace("exports = [\"api\"]", "exports = [\"api\", \"ui\"]");
    let f = findings(ROOT, &web);
    assert_eq!(f.iter().filter(|x| x.0 == "deps/private").count(), 1);
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
fn deny_help_asks_for_a_rule() {
    let h = helps("deps/deny", "`shared` uses `api`");
    assert_eq!(h.len(), 1, "{h:?}");
    assert!(h[0].contains("[[rule]]"));
}

#[test]
fn private_help_gives_the_export_command() {
    let h = helps("deps/private", "`shared`");
    assert_eq!(h.len(), 1, "{h:?}");
    assert!(h[0].contains("brickie design export set shared`"));
}

#[test]
fn cycles_are_found_per_level() {
    let f = findings(ROOT, WEB);
    assert!(has(&f, "deps/cycle", "root", "`core`, `web`"), "{f:#?}");
    assert!(has(&f, "deps/cycle", "root/web", "`api`, `ui`"), "{f:#?}");
}

#[test]
fn complexity_limits_only_tighten_across_levels() {
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
    // A nested rule cannot loosen the root limit for its children.
    let web = WEB.replace("cognitive = 10", "cognitive = 100");
    let root = ROOT.replace("cognitive = 20", "cognitive = 12");
    let f = findings(&root, &web);
    assert!(has(
        &f,
        "complexity/cognitive",
        "root/web",
        "`ui_fn` has cognitive 15 (limit 12)"
    ));
    // Children of web without a nested rule keep the root limit.
    assert!(has(
        &f,
        "complexity/cognitive",
        "root/web",
        "`api_fn` has cognitive 15 (limit 12)"
    ));
}
