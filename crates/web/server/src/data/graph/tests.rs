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
        level: brickie_check::config::Level::Error,
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
        accepted: false,
        accepted_reason: None,
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
