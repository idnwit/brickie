//! Check tests: `deps/*` rules.

use super::support::*;
use serde_json::json;

const OPEN: &str = "[[rule]]\nfrom = \"*\"\nto = \"*\"\naction = \"allow\"\n";

#[test]
fn deps_allow_passes() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks(
            r#"
[[rule]]
from = "*"
to = "port"
action = "allow"
[[rule]]
from = "app"
to = "*"
action = "allow"
[[rule]]
from = "engine"
to = "adapter"
action = "allow"
"#,
        ),
    );
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
    // Only the unmapped module remains; it is always an error.
    assert_eq!(o.code, 1);
    assert_eq!(o.json["summary"]["errors"], 1);
    let u = codes(&o, "bricks/unmapped");
    assert_eq!(u.len(), 1);
}

#[test]
fn deps_deny_fails() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks(&format!(
            r#"
{OPEN}
[[rule]]
from = "tag:core"
to = "tag:backend"
action = "deny"
reason = "core must not use adapter"
"#
        )),
    );
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0]["code"], "deps/deny");
    assert_eq!(d[0]["span"]["file"], "crates/eng/src/x.rs");
    assert_eq!(d[0]["span"]["line"], 2);
    assert_eq!(d[0]["span"]["col"], 14);
    assert!(
        d[0]["rule"]["name"]
            .as_str()
            .unwrap()
            .contains("core must not use adapter")
    );
    assert!(
        d[0]["rule"]["loc"]
            .as_str()
            .unwrap()
            .contains("brickie.toml:")
    );
}

#[test]
fn no_matching_rule_denies() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks(""));
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 5);
    assert!(d.iter().all(|d| d["code"] == "deps/deny"));
    assert!(d.iter().all(|d| d["rule"]["name"] == "no rule allows it"));
    // `* -> *` allow opens the level.
    let o = check(&w, &with_bricks(OPEN));
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
}

#[test]
fn deps_specificity() {
    let w = world(|_| {});
    // brick beats tag: the allow on `engine` wins over the deny on `tag:core`.
    let o = check(
        &w,
        &with_bricks(&format!(
            r#"
{OPEN}
[[rule]]
from = "tag:core"
to = "adapter"
action = "deny"
[[rule]]
from = "engine"
to = "adapter"
action = "allow"
"#
        )),
    );
    assert!(codes(&o, "deps").is_empty());
    // tag beats `*`.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[rule]]
from = "*"
to = "*"
action = "deny"
[[rule]]
from = "tag:core"
to = "tag:backend"
action = "allow"
[[rule]]
from = "*"
to = "port"
action = "allow"
[[rule]]
from = "app"
to = "*"
action = "allow"
"#,
        ),
    );
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
    // Equal specificity: deny wins whatever the order.
    let o = check(
        &w,
        &with_bricks(&format!(
            r#"
{OPEN}
[[rule]]
from = "tag:core"
to = "adapter"
action = "allow"
[[rule]]
from = "tag:core"
to = "adapter"
action = "deny"
[[rule]]
from = "tag:core"
to = "adapter"
action = "allow"
"#
        )),
    );
    assert_eq!(codes(&o, "deps/deny").len(), 1);
}

#[test]
fn deps_kinds_and_scope_filters() {
    let w = world(|c| {
        let f = &mut c["files"]["crates/eng/src/x.rs"]["edges"];
        f.as_array_mut()
            .unwrap()
            .push(edge("eng::x", "adp::y", "uses", "test", 4));
    });
    // Default scope is prod: the test edge is not matched; `kinds` restricts to implements.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[rule]]
from = "*"
to = "*"
scope = "all"
action = "allow"
[[rule]]
from = "engine"
to = "*"
kinds = ["implements"]
action = "deny"
[[rule]]
from = "engine"
to = "adapter"
action = "deny"
"#,
        ),
    );
    let d = codes(&o, "deps/deny");
    assert_eq!(d.len(), 2);
    assert!(d.iter().any(|d| {
        d["message"]
            .as_str()
            .unwrap()
            .contains("implements a trait of")
    }));
    assert!(
        d.iter()
            .all(|d| !d["message"].as_str().unwrap().contains("in tests"))
    );
    // scope = "test" matches the test edge only.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[rule]]
from = "*"
to = "*"
scope = "all"
action = "allow"
[[rule]]
from = "engine"
to = "adapter"
scope = "test"
action = "deny"
"#,
        ),
    );
    let d = codes(&o, "deps/deny");
    assert_eq!(d.len(), 1);
    assert!(d[0]["message"].as_str().unwrap().ends_with("in tests"));
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
