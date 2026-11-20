//! Unit tests of the graph helpers; the per-level aggregation is tested through the routes
//! in `web::tests::levels`.

use super::edges::End;
use super::level::cycle_hops;
use brickie_check::check as rules;
use brickie_check::config::{Brick, ComplexityRule, Config};
use std::collections::BTreeMap;

#[test]
fn cycle_hops_map_path_edges_to_the_finding() {
    use brickie_check::diag::{DiagEdge, Diagnostic};
    let hop = |f: &str, t: &str| DiagEdge {
        from: f.into(),
        to: t.into(),
        span: None,
    };
    let d = Diagnostic {
        id: "cyc1".into(),
        code: rules::DEPS_CYCLE.into(),
        message: String::new(),
        span: None,
        spans: vec![],
        rule: None,
        help: None,
        notes: vec![],
        bricks: vec!["a".into(), "b".into()],
        brick_level: Some("root".into()),
        edges: vec![hop("a", "b"), hop("b", "a"), hop("b", "zzz")],
    };
    let index: BTreeMap<End, usize> = [("a", 0), ("b", 1), ("c", 2)]
        .map(|(n, i)| (End::Child(n.into()), i))
        .into();
    let m = cycle_hops(&[d], "root", &index);
    assert_eq!(m.len(), 2);
    assert_eq!(m[&(0, 1)], vec!["cyc1".to_string()]);
    assert_eq!(m[&(1, 0)], vec!["cyc1".to_string()]);
}

#[test]
fn complexity_limits_use_the_most_specific_rule() {
    let rule = |bricks: &str, cognitive| -> ComplexityRule {
        let t = format!("bricks = \"{bricks}\"\ncognitive = {cognitive}");
        toml::from_str(&t).unwrap()
    };
    let brick = Brick {
        name: "core".into(),
        tags: vec!["engine".into()],
        members: vec![],
        path: None,
        desc: None,
        base: Default::default(),
        independent: false,
        sub: None,
    };
    let cfg = Config {
        bricks: vec![brick],
        complexity: vec![rule("*", 15), rule("core", 30)],
        ..Default::default()
    };
    let func: brickie_facts::Function = serde_json::from_value(serde_json::json!({
        "qname": "f", "name": "f", "module": null, "line": 1, "end_line": 5, "scope": "prod", "cognitive": 27,
        "cyclomatic": 1, "nesting": 1, "paths": 1, "params": 1, "lines": 5
    }))
    .unwrap();
    let limits = rules::complexity_limits(&cfg, &["core".to_string()], &func);
    assert_eq!(limits, vec![("cognitive", 27, 30)]);
}

#[test]
fn parent_links_project_onto_child_level_ghosts_without_cache() {
    use brickie_check::check::Loaded;
    use brickie_check::config::{Link, Rule, Scope, Sel};
    use brickie_facts::Cache;

    let leaf = |name: &str| Brick {
        name: name.into(),
        tags: vec![],
        members: vec![],
        path: None,
        desc: None,
        base: Default::default(),
        independent: false,
        sub: None,
    };
    let web_cfg = Config {
        path: vec!["web".into()],
        exports: vec!["api".into()],
        bricks: vec![leaf("api")],
        ..Default::default()
    };
    let web = Brick {
        sub: Some(Box::new(web_cfg)),
        ..leaf("web")
    };
    let root = Config {
        bricks: vec![leaf("client"), web, leaf("proto")],
        rules: vec![Rule {
            from: Sel::Any,
            to: Sel::Any,
            kinds: vec![],
            scope: Scope::All,
            reason: "allow".into(),
            loc: String::new(),
        }],
        links: vec![
            Link {
                from: "web".into(),
                to: "proto".into(),
                kinds: vec!["uses".into()],
                scope: Scope::All,
                reason: "web speaks proto".into(),
                loc: String::new(),
            },
            Link {
                from: "client".into(),
                to: "web/api".into(),
                kinds: vec!["uses".into()],
                scope: Scope::All,
                reason: "client calls web api".into(),
                loc: String::new(),
            },
        ],
        ..Default::default()
    };
    let loaded = Loaded {
        cfg: root,
        cache: Cache::default(),
        diagnostics: vec![],
        resolved: Default::default(),
        owners: Default::default(),
        issues: vec![],
        todo: Default::default(),
    };
    let lvs = super::levels(&loaded);
    let web_lv = lvs.iter().find(|l| l.path == "root/web").unwrap();

    // Ghosts exist even with empty measured cache.
    let g_client = web_lv
        .graph
        .bricks
        .iter()
        .position(|b| b.ghost && b.name == "root/client")
        .expect("inbound ghost for client");
    let g_proto = web_lv
        .graph
        .bricks
        .iter()
        .position(|b| b.ghost && b.name == "root/proto")
        .expect("outbound ghost for proto");
    let c_api = web_lv
        .graph
        .bricks
        .iter()
        .position(|b| !b.ghost && b.name == "api")
        .expect("child api");

    assert_eq!(web_lv.graph.bricks[g_client].tag, "inbound");
    assert_eq!(web_lv.graph.bricks[g_proto].tag, "outbound");
    assert_eq!(web_lv.boundary_links.len(), 2);

    let out = web_lv
        .boundary_links
        .iter()
        .find(|b| b.ghost == g_proto)
        .unwrap();
    assert!(!out.inbound);
    assert_eq!(out.child, None);
    assert_eq!(out.link.from, "web");

    let inc = web_lv
        .boundary_links
        .iter()
        .find(|b| b.ghost == g_client)
        .unwrap();
    assert!(inc.inbound);
    assert_eq!(inc.child, Some(c_api));
    assert_eq!(inc.link.to, "web/api");
}
