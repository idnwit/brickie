//! Interfaces on the two-level fixture of `config::tests` (root: web (composite exporting
//! `api` and, for tests, `ui`), core; web: api, ui, shared).

use super::*;
use crate::config::tests::{ROOT, WEB, cache, fixture, resolved};
use crate::diag::diagnostic;
use brickie_facts::Item;
use serde_json::{Value, json};

fn sym_edge(from: &str, to: &str, symbol: &str, scope: &str, line: u32) -> Value {
    json!({"from": format!("{from}::m"), "to": format!("{to}::m"), "kind": "uses",
           "scope": scope, "line": line, "col": 1, "end_col": 5, "symbol": symbol,
           "written": symbol})
}

fn item(pkg: &str, symbol: &str, exported: bool) -> Item {
    Item {
        symbol: symbol.into(),
        module: format!("{pkg}::m"),
        kind: "fn".into(),
        name: symbol.rsplit("::").next().unwrap().into(),
        signature: format!("pub fn {symbol}()"),
        doc: String::new(),
        line: 3,
        vis: "pub".into(),
        visible_in: pkg.into(),
        exported,
    }
}

fn names(i: &Interface) -> Vec<&str> {
    i.items.iter().map(|x| x.name.as_str()).collect()
}

/// api declares `Foo` (used by ui and core), `bar` (used only by a core test) and `dead`;
/// `hidden` is not exported from its crate. ui declares `view` (used by a web-api test),
/// shared declares `util`.
fn world() -> (tempfile::TempDir, crate::config::Config, Resolved, Cache) {
    let (dir, cfg) = fixture(ROOT, WEB);
    let res = resolved(dir.path(), &cfg).unwrap();
    let mut c = cache(
        &dir.path().canonicalize().unwrap(),
        &[
            (
                "web-ui",
                vec![sym_edge("web-ui", "web-api", "m::Foo::new", "prod", 4)],
                vec![],
            ),
            (
                "core",
                vec![
                    sym_edge("core", "web-api", "m::impl::Foo<'_>::run", "prod", 7),
                    sym_edge("core", "web-api", "m::bar", "test", 8),
                ],
                vec![],
            ),
            (
                "web-api",
                vec![sym_edge("web-api", "web-ui", "m::view", "test", 9)],
                vec![],
            ),
            ("web-shared", vec![], vec![]),
        ],
    );
    let add = |c: &mut Cache, dir: &str, items: Vec<Item>| {
        c.files
            .get_mut(&format!("{dir}/src/m.rs"))
            .unwrap()
            .items
            .extend(items);
    };
    add(
        &mut c,
        "crates/web/api",
        vec![
            item("web-api", "m::Foo", true),
            item("web-api", "m::bar", true),
            item("web-api", "m::dead", true),
            item("web-api", "m::hidden", false),
        ],
    );
    add(
        &mut c,
        "crates/web/ui",
        vec![item("web-ui", "m::view", true)],
    );
    add(
        &mut c,
        "crates/web/shared",
        vec![item("web-shared", "m::util", true)],
    );
    (dir, cfg, res, c)
}

#[test]
fn declared_follows_the_exports_chain() {
    let (_d, cfg, res, c) = world();
    let api = interface(&c, &cfg, &res, &["web".into(), "api".into()]);
    // Sorted by uses, unused last; `hidden` is not part of the crate API.
    assert_eq!(names(&api), ["Foo", "bar", "dead"]);
    let web = interface(&c, &cfg, &res, &["web".into()]);
    // Exported children only: api and ui, not shared.
    let mut n = names(&web);
    n.sort();
    assert_eq!(n, ["Foo", "bar", "dead", "view"]);
    assert_eq!(parse_path(&cfg, "root/web/api").unwrap(), ["web", "api"]);
    assert!(parse_path(&cfg, "root/nope").is_err());
}

#[test]
fn uses_are_grouped_at_the_brick_level() {
    let (_d, cfg, res, c) = world();
    let api = interface(&c, &cfg, &res, &["web".into(), "api".into()]);
    let foo = &api.items[0];
    let groups: Vec<&str> = foo.uses.iter().map(|g| g.brick.as_str()).collect();
    // Members (`Foo::new`) and impl items with generics count as uses of `Foo`.
    assert_eq!(groups, ["root/core", "root/web/ui"]);
    assert_eq!(foo.use_count(), 2);
    let web = interface(&c, &cfg, &res, &["web".into()]);
    let view = web.items.iter().find(|i| i.name == "view").unwrap();
    // A test inside `web` (web-api's test) counts, under the brick's own name.
    assert_eq!(view.uses[0].brick, "root/web (tests)");
    let foo = web.items.iter().find(|i| i.name == "Foo").unwrap();
    // ui -> api is inside `web`: only core's use is a use of web's interface.
    assert_eq!(foo.uses.len(), 1);
    assert_eq!(foo.uses[0].brick, "root/core");
}

#[test]
fn unused_items_are_reported_per_leaf() {
    let (_d, cfg, res, c) = world();
    let mut found: Vec<String> = unused(&c, &cfg, &res)
        .iter()
        .map(|f| diagnostic(f.as_ref()).message)
        .collect();
    found.sort();
    // `bar` has a test use, `view` too; `util` and `dead` have none.
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("`dead` is declared by `api`"));
    assert!(found[1].contains("`util` is declared by `shared`"));
    let help = unused(&c, &cfg, &res)
        .iter()
        .find_map(|f| diagnostic(f.as_ref()).help)
        .unwrap();
    assert!(help.contains("make it private, or remove it if nothing uses it"));
    assert!(!help.contains("pub(super)"));
}

#[test]
fn edge_sites_by_used_item() {
    let (_d, _cfg, res, c) = world();
    let v = edge_uses(&c, &res, &["core".into()], &["web".into()]);
    let got: Vec<(&str, usize)> = v.iter().map(|i| (i.name.as_str(), i.sites.len())).collect();
    assert_eq!(got, [("Foo", 1), ("bar", 1)]);
    assert_eq!(v[1].sites[0].scope, "test");
    assert_eq!(key("a::impl::Ctx<'_>::txt"), "a::Ctx::txt");
    assert!(covers("a::Foo", "a::Foo::x") && !covers("a::Foo", "a::Foobar"));
}
