//! Check tests: `deps/unused` (allow rules nothing uses) and its stable id.

use super::nested::{ROOT, WEB, tree};
use super::support::*;
use serde_json::Value;

const RULES: &str = r#"
[[brick]]
name = "ghost"
members = ["ghost"]

[[deps.rule]]
from = "port"
to = "app"
action = "allow"

[[deps.rule]]
from = "port"
to = "engine"
kinds = ["uses"]
scope = "test"
action = "allow"

[[deps.rule]]
from = "engine"
to = "ghost"
action = "allow"

[[deps.rule]]
from = "tag:core"
to = "app"
action = "allow"

[[deps.rule]]
from = "adapter"
to = "app"
action = "deny"
"#;

fn unused(o: &Out) -> Vec<&Value> {
    codes(o, "deps/unused")
}

#[test]
fn unused_needs_measured_ends_and_a_named_allow_rule() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks(RULES));
    let d = unused(&o);
    // port -> app and port -> engine (test) are unused; engine -> ghost is planned (ghost has
    // no module); the tag rule and the deny rule are not links.
    assert_eq!(d.len(), 2, "{}", o.stdout);
    assert!(d.iter().all(|d| d["level"] == "warn"));
    let msgs: Vec<&str> = d.iter().map(|d| d["message"].as_str().unwrap()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("`port` -> `app`") && m.contains("no edge in prod scope"))
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("no `uses` edge in test scope")),
        "{msgs:?}"
    );
    assert!(!msgs.iter().any(|m| m.contains("ghost")));
    assert_eq!(o.code, 0);
}

#[test]
fn unused_id_ignores_the_toml_layout() {
    let w = world(|_| {});
    let a = check(&w, &with_bricks(RULES));
    let moved = format!("# a comment\n\n\n{}", with_bricks(RULES));
    let b = check(&w, &moved);
    let ids = |o: &Out| -> Vec<String> {
        let mut v: Vec<String> = unused(o)
            .iter()
            .map(|d| d["id"].as_str().unwrap().into())
            .collect();
        v.sort();
        v
    };
    assert_eq!(ids(&a).len(), 2);
    assert_eq!(ids(&a), ids(&b));
    assert_ne!(unused(&a)[0]["span"]["line"], unused(&b)[0]["span"]["line"]);
}

#[test]
fn partly_measured_composite_is_planned_until_every_child_has_code() {
    let rule = "\n[[deps.rule]]\nfrom = \"web\"\nto = \"core\"\nkinds = [\"implements\"]\naction = \"allow\"\n";
    let w = tree();
    w.write("ws/crates/web/brickie.toml", WEB);
    let o = check(&w, &format!("{ROOT}{rule}"));
    assert_eq!(unused(&o).len(), 1, "{}{}", o.stdout, o.stderr);
    assert_eq!(unused(&o)[0]["brick_level"], "root");

    let ghost = format!("{WEB}\n[[brick]]\nname = \"ghost\"\nmembers = [\"web-ghost\"]\n");
    w.write("ws/crates/web/brickie.toml", &ghost);
    let o = check(&w, &format!("{ROOT}{rule}"));
    assert_eq!(o.code, 1, "{}", o.stderr); // the existing errors, not a config failure
    assert!(unused(&o).is_empty(), "{}", o.stdout);
}
