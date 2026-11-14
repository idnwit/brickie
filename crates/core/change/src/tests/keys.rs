//! Rules and limits are named by identity: missing, ambiguous and duplicate keys.

use super::*;
use crate::{ComplexitySpec, RuleKey, RulePatch, RuleSpec};

fn key(from: &str, to: &str) -> RuleKey {
    RuleKey {
        from: from.into(),
        to: to.into(),
        ..Default::default()
    }
}

fn rule(from: &str, to: &str, kinds: &[&str]) -> Op {
    Op::AddRule {
        file: "brickie.toml".into(),
        rule: RuleSpec {
            from: from.into(),
            to: to.into(),
            kinds: kinds.iter().map(|k| k.to_string()).collect(),
            action: "deny".into(),
            ..Default::default()
        },
    }
}

fn set(k: RuleKey, action: &str) -> Op {
    Op::SetRule {
        file: "brickie.toml".into(),
        key: k,
        set: RulePatch {
            action: Some(action.into()),
            ..Default::default()
        },
    }
}

#[test]
fn missing_rule_lists_the_candidates() {
    let ws = Ws::new();
    let e = format!(
        "{:#}",
        ws.apply(A, vec![set(key("b", "a"), "deny")]).unwrap_err()
    );
    assert!(
        e.contains("design/no-match: no rule b -> a in brickie.toml"),
        "{e}"
    );
    assert!(e.contains("a -> b (prod)"), "{e}");
}

#[test]
fn ambiguous_rule_asks_for_kind_and_scope() {
    let ws = Ws::new();
    ws.apply(A, vec![rule("a", "b", &["uses"])]).unwrap();
    let e = format!(
        "{:#}",
        ws.apply(A, vec![set(key("a", "b"), "deny")]).unwrap_err()
    );
    assert!(e.contains("2 rules a -> b") && e.contains("--kind"), "{e}");
    let narrow = RuleKey {
        kinds: Some(vec!["uses".into()]),
        ..key("a", "b")
    };
    ws.apply(A, vec![set(narrow, "allow")]).unwrap();
    let text = ws.read("brickie.toml");
    assert_eq!(text.matches("action = \"allow\"").count(), 2, "{text}");
}

#[test]
fn set_rule_edits_only_the_matched_rule_and_keeps_the_rest() {
    let ws = Ws::new();
    ws.apply(A, vec![rule("b", "a", &[])]).unwrap();
    ws.apply(A, vec![set(key("b", "a"), "allow")]).unwrap();
    let text = ws.read("brickie.toml");
    assert_eq!(text.matches("action = \"allow\"").count(), 2, "{text}");
}

#[test]
fn adding_an_existing_identity_is_refused() {
    let ws = Ws::new();
    let e = format!("{:#}", ws.apply(A, vec![rule("a", "b", &[])]).unwrap_err());
    assert!(e.contains("design/exists"), "{e}");
    let limit = |bricks: Option<&str>| Op::AddComplexity {
        file: "brickie.toml".into(),
        rule: ComplexitySpec {
            bricks: bricks.map(String::from),
            cognitive: Some(10),
            ..Default::default()
        },
    };
    ws.apply(A, vec![limit(None)]).unwrap();
    let e = format!("{:#}", ws.apply(A, vec![limit(Some("*"))]).unwrap_err());
    assert!(e.contains("design/exists"), "{e}");
    let none = Op::RemoveComplexity {
        file: "brickie.toml".into(),
        bricks: Some("a".into()),
    };
    let e = format!("{:#}", ws.apply(A, vec![none]).unwrap_err());
    assert!(e.contains("design/no-match: no limit for 'a'"), "{e}");
}
