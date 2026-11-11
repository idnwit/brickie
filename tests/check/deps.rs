//! Check tests: `deps/*` rules.

use super::support::*;
use serde_json::json;

#[test]
fn deps_allow_passes() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks(
            r#"
[deps]
default = "deny"
[[deps.rule]]
from = "*"
to = "port"
action = "allow"
[[deps.rule]]
from = "app"
to = "*"
action = "allow"
[[deps.rule]]
from = "engine"
to = "adapter"
action = "allow"
"#,
        ),
    );
    assert!(codes(&o, "deps").is_empty(), "{}", o.stdout);
    // Only the unmapped-module warning remains.
    assert_eq!(o.code, 0);
    assert_eq!(o.json["summary"]["errors"], 0);
    assert_eq!(codes(&o, "bricks/unmapped").len(), 1);
}

#[test]
fn deps_deny_fails() {
    let w = world(|_| {});
    let o = check(
        &w,
        &with_bricks(
            r#"
[[deps.rule]]
from = "tag:core"
to = "tag:backend"
action = "deny"
reason = "core must not use adapter"
help = "go through the port"
"#,
        ),
    );
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0]["code"], "deps/deny");
    assert_eq!(d[0]["level"], "error");
    assert_eq!(d[0]["help"], "go through the port");
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
            .ends_with("brickie.toml:18")
    );
}

#[test]
fn deps_default_deny() {
    let w = world(|_| {});
    let o = check(&w, &with_bricks("[deps]\ndefault = \"deny\"\n"));
    assert_eq!(o.code, 1);
    let d = codes(&o, "deps");
    assert_eq!(d.len(), 5);
    assert!(d.iter().all(|d| d["code"] == "deps/default"));
    // Without a [deps] section, nothing is denied.
    let o = check(&w, &with_bricks(""));
    assert!(codes(&o, "deps").is_empty());
    assert_eq!(o.code, 0);
}

#[test]
fn deps_specificity() {
    let w = world(|_| {});
    // brick beats tag: the allow on `engine` wins over the deny on `tag:core`.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[deps.rule]]
from = "tag:core"
to = "adapter"
action = "deny"
[[deps.rule]]
from = "engine"
to = "adapter"
action = "allow"
"#,
        ),
    );
    assert!(codes(&o, "deps").is_empty());
    // tag beats `*`.
    let o = check(
        &w,
        &with_bricks(
            r#"
[[deps.rule]]
from = "*"
to = "*"
action = "deny"
[[deps.rule]]
from = "tag:core"
to = "tag:backend"
action = "allow"
[[deps.rule]]
from = "*"
to = "port"
action = "allow"
[[deps.rule]]
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
        &with_bricks(
            r#"
[[deps.rule]]
from = "tag:core"
to = "adapter"
action = "allow"
[[deps.rule]]
from = "tag:core"
to = "adapter"
action = "deny"
[[deps.rule]]
from = "tag:core"
to = "adapter"
action = "allow"
"#,
        ),
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
[[deps.rule]]
from = "engine"
to = "*"
kinds = ["implements"]
action = "deny"
[[deps.rule]]
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
[[deps.rule]]
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
        c["bricks"]["module_brick"]["eng::other"] = json!("engine");
    });
    let o = check(&w, &with_bricks("[deps]\ndefault = \"deny\"\n"));
    assert_eq!(codes(&o, "deps").len(), 5);
}
