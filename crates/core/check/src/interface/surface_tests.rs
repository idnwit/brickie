//! Surface uses on the fixture of `tests`: `Foo` is used by other bricks; the types, traits and
//! fields its surface names are used with it, transitively, within one crate.

use super::tests::world;
use super::*;
use brickie_facts::Item;

fn typed(pkg: &str, name: &str, kind: &str, surface: &[&str]) -> Item {
    Item {
        kind: kind.into(),
        name: name.into(),
        signature: format!("pub {kind} {name}"),
        surface: surface.iter().map(|s| s.to_string()).collect(),
        ..super::tests::item(pkg, &format!("m::{name}"), true)
    }
}

#[test]
fn surface_names_are_used_transitively() {
    let (_d, cfg, res, mut c) = world();
    let f = c.files.get_mut("crates/web/api/src/m.rs").unwrap();
    // a: signature (parameter, return, bound); b: pub field; c: implemented trait.
    f.items
        .iter_mut()
        .find(|i| i.name == "Foo")
        .unwrap()
        .signature = "pub fn Foo<T: Elem>(r: Rank) -> Rows<Other>".into();
    f.items.extend([
        typed("web-api", "Rank", "struct", &["Marker"]),
        typed("web-api", "Elem", "trait", &[]),
        typed("web-api", "Rows", "struct", &["pub inner: Deep"]),
        typed("web-api", "Deep", "struct", &[]),
        typed("web-api", "Marker", "trait", &[]),
        typed("web-api", "Lonely", "struct", &["Other"]),
    ]);
    // The same name in another crate is not reached.
    c.files
        .get_mut("crates/web/ui/src/m.rs")
        .unwrap()
        .items
        .push(typed("web-ui", "Other", "struct", &[]));
    let found: Vec<String> = unused(&c, &cfg, &res)
        .iter()
        .map(|f| crate::diag::diagnostic(f.as_ref()).message)
        .collect();
    let mut names: Vec<&str> = found.iter().filter_map(|m| m.split('`').nth(1)).collect();
    names.sort();
    assert_eq!(names, ["Lonely", "Other", "dead", "util"], "{found:?}");
    // At the level of `web` the group names the brick of the used item.
    let web = interface(&c, &cfg, &res, &["web".into()]);
    let deep = web.items.iter().find(|i| i.name == "Deep").unwrap();
    assert_eq!(deep.uses.len(), 1);
    assert_eq!(deep.uses[0].brick, "root/web (via surface)");
    let s = &deep.uses[0].sites[0];
    assert_eq!((s.kind.as_str(), s.scope.as_str()), ("surface", "prod"));
    assert_eq!((s.written.as_str(), s.line), ("Rows", 3));
    // A use of its own wins: no surface group on `Foo`.
    let foo = web.items.iter().find(|i| i.name == "Foo").unwrap();
    assert!(foo.uses.iter().all(|g| !g.brick.contains("surface")));
}
