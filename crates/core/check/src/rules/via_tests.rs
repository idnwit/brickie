//! `via` on dependency rules in a hexagonal shape: `core` plays the ui, the composite `web`
//! plays `application` with `api` as ports-in, `ui` as ports-out and `shared` as usecases.

use crate::config::tests::{cache, edge, fixture, resolved};
use crate::config::{Action, Brick, DepRule, Sel, decide};
use crate::diag::diagnostic;

const WEB: &str = r#"
exports = ["api", "ui"]
[[brick]]
name = "api"
members = ["web-api"]
[[brick]]
name = "ui"
members = ["web-ui"]
[[brick]]
name = "shared"
members = ["web-shared"]
"#;

/// `open`: a `* -> *` allow rule opens the level; otherwise no match denies.
fn root(open: bool, rules: &str) -> String {
    let all = if open {
        "[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n"
    } else {
        ""
    };
    format!(
        r#"
[[brick]]
name = "web"
path = "crates/web"
[[brick]]
name = "core"
members = ["core"]
{all}{rules}"#
    )
}

fn rule(via: &str, action: &str) -> String {
    format!("[[rule]]\nfrom = \"core\"\nto = \"web\"\n{via}\naction = \"{action}\"\n")
}

/// (code, message) of the findings for `core` using web's api, ui and shared.
fn run(root: &str) -> Vec<(String, String)> {
    let (dir, cfg) = fixture(root, WEB);
    let res = resolved(dir.path(), &cfg).unwrap();
    let ws = dir.path().canonicalize().unwrap();
    let edges = vec![
        edge("core", "web-api", "prod"),
        edge("core", "web-ui", "prod"),
        edge("core", "web-shared", "prod"),
    ];
    let c = cache(&ws, &[("core", edges, vec![])]);
    let mut out: Vec<_> = super::evaluate(&c, &cfg, &res, dir.path())
        .iter()
        .map(|f| {
            let d = diagnostic(f.as_ref());
            (d.code, d.message)
        })
        .collect();
    out.sort();
    out
}

fn count(f: &[(String, String)], code: &str, needle: &str) -> usize {
    f.iter()
        .filter(|(c, m)| c == code && m.contains(needle))
        .count()
}

#[test]
fn via_allow_under_no_match() {
    let f = run(&root(false, &rule("via = [\"api\"]", "allow")));
    assert_eq!(count(&f, "deps/deny", "via `api`"), 0, "{f:#?}");
    assert_eq!(count(&f, "deps/deny", "`core` uses `web` via `ui`"), 1);
    assert_eq!(count(&f, "deps/deny", "via `shared`"), 1);
    assert_eq!(count(&f, "deps/private", "`shared`"), 1, "{f:#?}");
    assert_eq!(f.len(), 3, "{f:#?}");
}

#[test]
fn via_deny_under_open_level_leaves_hidden_to_private() {
    let f = run(&root(true, &rule("via = [\"ui\"]", "deny")));
    assert_eq!(
        count(&f, "deps/deny", "`core` uses `web` via `ui`"),
        1,
        "{f:#?}"
    );
    assert_eq!(count(&f, "deps/private", "`shared`"), 1);
    assert_eq!(f.len(), 2, "{f:#?}");
}

#[test]
fn via_rule_beats_plain_rule_and_deny_wins_a_tie() {
    let rules = format!("{}{}", rule("", "allow"), rule("via = [\"ui\"]", "deny"));
    let f = run(&root(false, &rules));
    assert_eq!(count(&f, "deps/deny", "via `ui`"), 1, "{f:#?}");
    assert_eq!(count(&f, "deps/deny", "via `api`"), 0, "{f:#?}");
    let allow = rule("via = [\"api\"]", "allow");
    let deny = rule("via = [\"api\"]", "deny");
    for r in [format!("{allow}{deny}"), format!("{deny}{allow}")] {
        let f = run(&root(true, &r));
        assert_eq!(count(&f, "deps/deny", "via `api`"), 1, "{f:#?}");
    }
}

#[test]
fn leaf_target_never_matches_a_via_rule() {
    let leaf = Brick {
        name: "leaf".into(),
        members: vec![],
        tags: vec![],
        path: None,
        desc: None,
        independent: false,
        base: Default::default(),
        sub: None,
    };
    let r = DepRule {
        from: Sel::Any,
        to: Sel::Brick("leaf".into()),
        via: Some(vec!["x".into()]),
        kinds: vec![],
        scope: Default::default(),
        action: Action::Allow,
        reason: None,
        loc: String::new(),
    };
    assert!(!r.enters(None));
    assert!(r.enters(Some("x")));
    assert!(!r.enters(Some("y")));
    let level = crate::config::Config {
        rules: vec![r],
        ..Default::default()
    };
    // A leaf has no entered child, so the via rule never decides.
    assert!(
        decide(&level, &leaf, &leaf, None, "uses", "prod")
            .rule
            .is_none()
    );
}
