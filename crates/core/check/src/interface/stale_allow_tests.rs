//! `allow(interface/unused)` directives on the fixture of `tests`: `dead` and `util` are unused,
//! `Foo` and `bar` are used.

use super::tests::world;
use super::*;
use crate::diag::diagnostic;
use brickie_facts::Allow;

fn allow(c: &mut Cache, name: &str, codes: &[&str], reason: &str) {
    let item = c
        .files
        .values_mut()
        .flat_map(|f| f.items.iter_mut())
        .find(|i| i.name == name)
        .unwrap();
    item.allow = Some(Allow {
        line: 2,
        codes: codes.iter().map(|c| c.to_string()).collect(),
        reason: reason.into(),
    });
}

/// `(code, message)` of every finding.
fn found(c: &Cache) -> Vec<(String, String)> {
    let (_d, cfg, res, _) = world();
    unused(c, &cfg, &res)
        .iter()
        .map(|f| {
            let d = diagnostic(f.as_ref());
            (d.code, d.message)
        })
        .collect()
}

#[test]
fn directive_exempts_an_unused_item() {
    let (_d, cfg, res, mut c) = world();
    allow(&mut c, "dead", &["interface/unused"], "build.rs picks it");
    let f = found(&c);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].1.contains("`util`"));
    let api = interface(&c, &cfg, &res, &["web".into(), "api".into()]);
    let dead = api.items.iter().find(|i| i.name == "dead").unwrap();
    assert!(dead.allowed && !dead.unused());
    assert_eq!(api.unused(), 0);
}

#[test]
fn several_codes_are_listed() {
    let (_d, _cfg, _res, mut c) = world();
    allow(&mut c, "dead", &["interface/unused", "other/code"], "why");
    let f = found(&c);
    // `other/code` is unknown: one stale finding besides `util`.
    let stale: Vec<_> = f
        .iter()
        .filter(|(k, _)| k == "config/stale-allow")
        .collect();
    assert_eq!(stale.len(), 1, "{f:?}");
    assert!(stale[0].1.contains("`other/code` cannot be exempted"));
    assert!(!f.iter().any(|(_, m)| m.contains("`dead`")));
}

#[test]
fn missing_reason_is_stale_and_does_not_exempt() {
    let (_d, _cfg, _res, mut c) = world();
    allow(&mut c, "dead", &["interface/unused"], "");
    let f = found(&c);
    let stale: Vec<_> = f
        .iter()
        .filter(|(k, _)| k == "config/stale-allow")
        .collect();
    assert_eq!(stale.len(), 1, "{f:?}");
    assert!(stale[0].1.contains("no reason") && !stale[0].1.contains("not unused"));
    assert!(f.iter().any(|(_, m)| m.contains("`dead`")));
}

#[test]
fn directive_on_a_used_item_is_stale() {
    let (_d, _cfg, _res, mut c) = world();
    allow(&mut c, "bar", &["interface/unused"], "why");
    let f = found(&c);
    let stale: Vec<_> = f
        .iter()
        .filter(|(k, _)| k == "config/stale-allow")
        .collect();
    assert_eq!(stale.len(), 1, "{f:?}");
    assert!(stale[0].1.contains("`bar` is not unused anyway"));
}

#[test]
fn directive_on_a_published_item_is_stale() {
    let (_d, cfg, res, mut c) = world();
    c.crates
        .iter_mut()
        .for_each(|k| k.publish = k.id == "web-api");
    allow(&mut c, "dead", &["interface/unused"], "why");
    let f: Vec<_> = unused(&c, &cfg, &res)
        .iter()
        .map(|f| diagnostic(f.as_ref()))
        .filter(|d| d.code == "config/stale-allow")
        .collect();
    assert_eq!(f.len(), 1);
}

#[test]
fn unknown_code_only_is_stale() {
    let (_d, _cfg, _res, mut c) = world();
    allow(&mut c, "dead", &["deps/cycle"], "why");
    let f = found(&c);
    assert!(
        f.iter()
            .any(|(k, m)| k == "config/stale-allow" && m.contains("`deps/cycle`"))
    );
    assert!(f.iter().any(|(_, m)| m.contains("`dead`")));
}
