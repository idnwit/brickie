//! Check tests: weight-bounded acceptance and orphan accepts.

use super::support::*;

const RULE: &str = "[[complexity]]\nbricks = \"tag:core\"\ncognitive = 15\n";

/// The id of the `busy` complexity finding (weight 20) in a fresh world.
fn busy(w: &World) -> String {
    let o = check(w, &with_bricks(RULE));
    let d = codes(&o, "complexity/cognitive");
    assert_eq!(d.len(), 1);
    d[0]["id"].as_str().unwrap().to_string()
}

fn accept(w: &World, id: &str, extra: &str) -> Out {
    w.write(
        "brickie.review.toml",
        &format!("[[accept]]\nid = \"{id}\"\nreason = \"r\"\n{extra}"),
    );
    check(w, &with_bricks(RULE))
}

#[test]
fn weight_bounds_the_acceptance() {
    let w = world(|_| {});
    let id = busy(&w);
    for extra in ["weight = 20\n", "weight = 25\n", ""] {
        let o = accept(&w, &id, extra);
        assert_eq!(o.code, 0, "{extra}: {}", o.stdout);
        assert_eq!(o.json["summary"]["accepted"], 1);
    }
    let o = accept(&w, &id, "weight = 19\n");
    assert_eq!(o.code, 1, "{}", o.stdout);
    let d = codes(&o, "complexity/cognitive")[0];
    assert_eq!(d["level"], "error");
    assert_eq!(d["accepted"], false);
    assert_eq!(d["notes"][0]["text"], "worse than accepted: 19 -> 20");
}

#[test]
fn unknown_accept_is_an_orphan_warning() {
    let w = world(|_| {});
    let o = accept(&w, "deadbeef", "");
    let d = codes(&o, "review/orphan");
    assert_eq!(d.len(), 1, "{}", o.stdout);
    assert_eq!(d[0]["level"], "warn");
    assert_eq!(
        d[0]["message"],
        "accepted finding `deadbeef` no longer exists"
    );
    assert_eq!(d[0]["span"]["line"], 1);
    assert_eq!(o.code, 1, "the complexity finding is still an error");
    let id = busy(&w);
    let o = accept(&w, &id, "");
    assert!(codes(&o, "review/orphan").is_empty());
    assert_eq!(o.code, 0);
}

#[test]
fn no_orphan_while_facts_are_stale() {
    let w = world(|_| {});
    let cfg = w.save(&with_bricks(RULE));
    w.write("ws/crates/eng/src/x.rs", "// edited\n");
    w.write(
        "brickie.review.toml",
        "[[accept]]\nid = \"deadbeef\"\nreason = \"r\"\n",
    );
    let o = run(&cfg, &["--format", "json"]);
    assert!(!codes(&o, "stale/file").is_empty());
    assert!(codes(&o, "review/orphan").is_empty(), "{}", o.stdout);
}
