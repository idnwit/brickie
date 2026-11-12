//! Rules the change adds: planned edges (no measured edge backs them), rules without a
//! reason, and the link check.

use super::drift::cache;
use super::*;
use crate::{Link, RuleSpec, TaskKind};

fn rule(from: &str, to: &str, reason: Option<&str>) -> Op {
    Op::AddRule {
        file: "brickie.toml".into(),
        rule: RuleSpec {
            from: from.into(),
            to: to.into(),
            scope: Some("all".into()),
            action: "allow".into(),
            reason: reason.map(String::from),
            ..RuleSpec::default()
        },
    }
}

#[test]
fn planned_edges_and_rules_without_reason() {
    let ws = Ws::new();
    cache(&ws, &ws.dir.path().display().to_string());
    let c = ws.open();
    c.record(H, add("c", &["pc"]), None).unwrap();
    // pc -> pa is measured: backed, but it has no reason.
    c.record(A, rule("c", "a", None), None).unwrap();
    // Nothing in pa uses pc: a planned edge.
    let planned = c.record(H, rule("a", "c", Some("soon")), None).unwrap();
    let edges = c.planned_edges().unwrap();
    assert_eq!(edges.len(), 1, "{edges:?}");
    let e = &edges[0];
    assert_eq!(
        (e.level.as_str(), e.from.as_str(), e.to.as_str(), e.rule),
        ("root", "a", "c", 2)
    );
    assert_eq!(
        (e.seq, e.by, e.scope.as_str()),
        (Some(planned.seq), Some(H), "all")
    );
    let todo = c.todo().unwrap();
    let of = |k: TaskKind| todo.iter().filter(|t| t.kind == k).collect::<Vec<_>>();
    assert_eq!(of(TaskKind::Link).len(), 1, "{todo:?}");
    let reason = of(TaskKind::Reason);
    assert_eq!(reason.len(), 1, "{todo:?}");
    assert!(reason[0].text.contains("c -> a"), "{todo:?}");
    let plan = c.publish_plan().unwrap();
    assert_eq!(plan.warnings.len(), 1, "{:?}", plan.warnings);
    assert!(plan.warnings[0].starts_with("change/no-reason"));
}

#[test]
fn link_check_refuses_links_that_change_nothing() {
    let ws = Ws::new();
    let c = ws.open();
    let link = |from, to, scope, action| Link {
        from,
        to,
        kinds: &[],
        scope,
        action,
    };
    let same = c.link_check(&[], &link("a", "b", "prod", "allow")).unwrap();
    assert!(same.redundant && same.why.contains("same rule"), "{same:?}");
    let kinds = ["uses".to_string()];
    let narrower = Link {
        kinds: &kinds,
        ..link("a", "b", "prod", "allow")
    };
    let n = c.link_check(&[], &narrower).unwrap();
    assert!(
        n.redundant && n.why.starts_with("already allowed by"),
        "{n:?}"
    );
    let all = c.link_check(&[], &link("a", "b", "all", "allow")).unwrap();
    assert!(
        !all.redundant && all.why == "would allow 3 of 6 edges",
        "{all:?}"
    );
    let denied = c.link_check(&[], &link("b", "a", "all", "deny")).unwrap();
    assert!(denied.redundant && denied.rule.is_none(), "{denied:?}");
    assert!(
        c.link_check(&[], &link("a", "tag:none", "all", "allow"))
            .is_err()
    );
    assert!(c.link_check(&[], &link("a", "b", "both", "allow")).is_err());
}
