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
        group: None,
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
        group: None,
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

#[test]
fn group_level_view_emits_units_internal_edges_heat_and_projected_links() {
    use brickie_check::check::Loaded;
    use brickie_check::config::{Group, Link, Rule, Scope, Sel};
    use brickie_facts::{Cache, FileData};
    use std::collections::BTreeMap;

    let leaf = |name: &str, group: Option<&str>| Brick {
        name: name.into(),
        tags: vec![],
        members: vec![name.into()],
        path: None,
        desc: None,
        base: Default::default(),
        independent: false,
        group: group.map(str::to_string),
        sub: None,
    };

    // Composite `auth` inside group `services`, plus leaf `billing` inside `services`, plus `api`.
    let auth_sub = Config {
        path: vec!["auth".into()],
        exports: vec!["jwt".into()],
        bricks: vec![leaf("jwt", None)],
        ..Default::default()
    };
    let mut auth = leaf("auth", Some("services"));
    auth.sub = Some(Box::new(auth_sub));
    let billing = leaf("billing", Some("services"));
    let api = leaf("api", None);

    let root = Config {
        bricks: vec![api, auth, billing],
        groups: vec![Group {
            name: "services".into(),
            tags: vec!["domain".into()],
            desc: Some("Domain services".into()),
            loc: String::new(),
        }],
        rules: vec![Rule {
            from: Sel::Any,
            to: Sel::Any,
            kinds: vec![],
            scope: Scope::All,
            reason: "allow".into(),
            loc: String::new(),
        }],
        links: vec![Link {
            from: "api".into(),
            to: "services".into(),
            kinds: vec!["uses".into()],
            scope: Scope::All,
            reason: "api calls services".into(),
            loc: String::new(),
        }],
        complexity: vec![toml::from_str("bricks = \"services\"\ncognitive = 10").unwrap()],
        ..Default::default()
    };

    let mut files = BTreeMap::new();
    let file_data: FileData = serde_json::from_value(serde_json::json!({
        "sha256": "",
        "compiled": true,
        "size": 100,
        "lines": 20,
        "scope": "prod",
        "module": "billing",
        "functions": [{
            "qname": "billing::charge", "name": "charge", "module": "billing",
            "line": 1, "end_line": 20, "scope": "prod",
            "cognitive": 20, "cyclomatic": 1, "nesting": 1, "paths": 1, "params": 1, "lines": 20
        }],
        "edges": [{
            "from": "auth::jwt", "to": "billing", "kind": "uses", "scope": "prod",
            "line": 3, "col": 1, "end_col": 5, "symbol": "charge", "written": "charge"
        }],
        "items": []
    }))
    .unwrap();
    files.insert("billing/src/lib.rs".into(), file_data);
    let cache = Cache {
        files,
        ..Default::default()
    };

    let mut resolved = brickie_check::config::Resolved::default();
    resolved.paths.insert("api".into(), vec!["api".into()]);
    resolved
        .paths
        .insert("auth::jwt".into(), vec!["auth".into(), "jwt".into()]);
    resolved
        .paths
        .insert("billing".into(), vec!["billing".into()]);

    let loaded = Loaded {
        cfg: root,
        cache,
        diagnostics: vec![],
        resolved,
        owners: Default::default(),
        issues: vec![],
        todo: Default::default(),
    };

    let lvs = super::levels(&loaded);
    // Only root and root/auth levels (no level for group `services`).
    assert_eq!(lvs.len(), 2);
    let root_lv = &lvs[0];
    // Two logical units at root: `api` and group `services` (not 3 bricks).
    assert_eq!(root_lv.graph.bricks.len(), 2);
    let grp = root_lv
        .graph
        .bricks
        .iter()
        .find(|b| b.name == "services")
        .expect("services group");
    assert!(grp.is_group);
    assert_eq!(grp.group_members.len(), 2);
    assert!(grp.group_members[0].is_composite);
    assert_eq!(grp.group_members[1].name, "billing");
    assert!((grp.worst_ratio - 2.0).abs() < 1e-9);
    assert!((grp.group_members[1].worst_ratio - 2.0).abs() < 1e-9);
    // Intra-group edge auth -> billing is preserved in internal_edges, not dropped.
    assert_eq!(grp.internal_edges.len(), 1);
    assert_eq!(grp.internal_edges[0].from_member, "auth");
    assert_eq!(grp.internal_edges[0].to_member, "billing");

    // Zoomed into grouped composite `root/auth`: projected link `api -> services` appears as
    // inbound ghost `root/api`, and ghost index offset works with unit count.
    let auth_lv = lvs.iter().find(|l| l.path == "root/auth").unwrap();
    let g_api = auth_lv
        .graph
        .bricks
        .iter()
        .position(|b| b.ghost && b.name == "root/api")
        .expect("projected inbound ghost for api");
    assert!(
        auth_lv
            .boundary_links
            .iter()
            .any(|bl| bl.inbound && bl.ghost == g_api)
    );
}
