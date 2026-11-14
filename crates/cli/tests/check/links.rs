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
fn unused_is_any_named_allow_rule_no_edge_uses() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks(RULES));
    let d = unused(&o);
    // port -> app, port -> engine (test) and engine -> ghost (ghost has no module) are unused;
    // the tag rule and the deny rule are not links.
    assert_eq!(d.len(), 3, "{}", o.stdout);
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
    assert!(msgs.iter().any(|m| m.contains("`ghost`")), "{msgs:?}");
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
    assert_eq!(ids(&a).len(), 3);
    assert_eq!(ids(&a), ids(&b));
    assert_ne!(unused(&a)[0]["span"]["line"], unused(&b)[0]["span"]["line"]);
}

#[test]
fn rule_towards_a_partly_measured_composite_is_unused_without_an_edge() {
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
    assert_eq!(unused(&o).len(), 1, "{}", o.stdout);
}
