//! Check tests: `deps/*` rules.

use super::support::*;
use serde_json::json;

const OPEN: &str = "[[rule]]\nfrom = \"*\"\nto = \"*\"\nreason = \"open\"\n";

fn link(from: &str, to: &str, extra: &str) -> String {
    format!("[[link]]\nfrom = \"{from}\"\nto = \"{to}\"\n{extra}reason = \"needed\"\n")
}

/// Links covering all five edges of the world.
fn all_links() -> String {
    [
        link("engine", "adapter", ""),
        link("engine", "port", ""),
        link("adapter", "port", ""),
        link("app", "engine", ""),
        link("app", "adapter", ""),
    ]
    .join("")
}

#[test]
fn deps_allow_passes() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks(&format!(
            r#"
[[rule]]
from = "*"
to = "port"
reason = "everyone may use the port"
[[rule]]
from = "app"
to = "*"
reason = "app may use anything"
[[rule]]
from = "engine"
to = "adapter"
reason = "engine drives adapters"
{}"#,
            all_links()
        )),
    );
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
    // Only the unmapped module remains; it is always an error.
    assert_eq!(o.code, 1);
    assert_eq!(o.json["summary"]["errors"], 1);
    let u = codes(&o, "bricks/unmapped");
    assert_eq!(u.len(), 1);
}

#[test]
fn unlinked_edge_fails() {
    let w = world(|_| {});
    // Everything is allowed by a rule, but engine -> adapter has no link.
    let links = [
        link("engine", "port", ""),
        link("adapter", "port", ""),
        link("app", "engine", ""),
        link("app", "adapter", ""),
    ]
    .join("");
    let o = check(&w, &with_bricks(&format!("{OPEN}{links}")));
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0]["code"], "deps/unlinked");
    assert_eq!(d[0]["span"]["file"], "crates/eng/src/x.rs");
    assert_eq!(d[0]["span"]["line"], 2);
    assert_eq!(d[0]["span"]["col"], 14);
    assert!(
        d[0]["help"]
            .as_str()
            .unwrap()
            .contains("brickie design link add engine adapter --kind uses --scope prod")
    );
    assert!(
        d[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .contains("brickie.toml")
    );
}

#[test]
fn link_without_allowing_rule_is_unlinked_and_denied() {
    let w = world(|_| {});
    // Rules allow everything but engine -> adapter, which is linked anyway.
    let rules = r#"
[[rule]]
from = "*"
to = "port"
reason = "port is open"
[[rule]]
from = "app"
to = "*"
reason = "app may use anything"
"#;
    let cfg = with_bricks(&format!("{rules}{}", all_links()));
    let o = check(&w, &cfg);
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 1, "{}", o.stdout);
    assert_eq!(d[0]["code"], "deps/unlinked");
    assert_eq!(d[0]["span"]["line"], 2);
    // `design check` judges the links from the design files alone.
    let path = w.save(&cfg);
    let c = std::process::Command::new(BIN)
        .args(["design", "check", "--config"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(c.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&c.stdout).contains("design/link-denied"));
}

#[test]
fn no_link_leaves_every_edge_unlinked() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks(OPEN));
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 5);
    assert!(d.iter().all(|d| d["code"] == "deps/unlinked"));
    assert!(d.iter().all(|d| d["rule"]["name"] == "no link covers it"));
    // Links plus an open rule cover all of them.
    let o = check(&w, &with_bricks(&format!("{OPEN}{}", all_links())));
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
}

#[test]
fn deps_kinds_and_scope_filters() {
    let w = world(|c| {
        let f = &mut c["files"]["crates/eng/src/x.rs"]["edges"];
        f.as_array_mut()
            .unwrap()
            .push(edge("eng::x", "adp::y", "uses", "test", 4));
    });
    let rest = format!(
        "[[rule]]\nfrom = \"*\"\nto = \"*\"\nscope = \"all\"\nreason = \"open\"\n{}{}{}{}",
        link("engine", "port", "kinds = [\"implements\"]\n"),
        link("adapter", "port", ""),
        link("app", "engine", ""),
        link("app", "adapter", ""),
    );
    // No engine -> adapter link: both the prod and the test edge are unlinked.
    let o = check(&w, &with_bricks(&rest));
    assert_eq!(codes(&o, "deps/unlinked").len(), 2, "{}", o.stdout);
    // A prod-only link leaves the test edge unlinked.
    let o = check(
        &w,
        &with_bricks(&format!(
            "{rest}{}",
            link("engine", "adapter", "scope = \"prod\"\n")
        )),
    );
    let d = codes(&o, "deps/unlinked");
    assert_eq!(d.len(), 1, "{}", o.stdout);
    assert!(d[0]["message"].as_str().unwrap().ends_with("in tests"));
    // A link for another kind does not cover a `uses` edge.
    let o = check(
        &w,
        &with_bricks(&format!(
            "{rest}{}",
            link("engine", "adapter", "kinds = [\"implements\"]\n")
        )),
    );
    assert_eq!(codes(&o, "deps/unlinked").len(), 2, "{}", o.stdout);
    // A link with kinds `uses` and scope all covers both.
    let o = check(
        &w,
        &with_bricks(&format!(
            "{rest}{}",
            link("engine", "adapter", "kinds = [\"uses\"]\nscope = \"all\"\n")
        )),
    );
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
}

#[test]
fn edges_inside_a_brick_are_ignored() {
    let w = world(|c| {
        c["files"]["crates/eng/src/x.rs"]["edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("eng::x", "eng::other", "uses", "prod", 4));
        c["modules"].as_array_mut().unwrap().push(
            json!({"id": "eng::other", "crate": "eng", "file": "crates/eng/src/x.rs", "line": 3}),
        );
    });
    let o = check(&w, &with_bricks(""));
    assert_eq!(codes(&o, "deps").len(), 5);
}
