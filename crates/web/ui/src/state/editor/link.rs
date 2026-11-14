//! Typed links: the four port types of a brick (a dependency kind, or deny), the rule a drop
//! adds, and which rule selectors name a brick.

use brickie_web_api::BrickView;
use brickie_web_api::design::{Op, RulePatch, RuleSpec};

/// The type of a link port: it fixes the rule's kind, and `Deny` its action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Uses,
    Implements,
    Reexports,
    Deny,
}

pub const LINK_TYPES: [LinkType; 4] = [
    LinkType::Uses,
    LinkType::Implements,
    LinkType::Reexports,
    LinkType::Deny,
];

pub const SCOPES: [&str; 3] = ["prod", "test", "all"];

impl LinkType {
    pub fn name(self) -> &'static str {
        match self {
            LinkType::Uses => "uses",
            LinkType::Implements => "implements",
            LinkType::Reexports => "reexports",
            LinkType::Deny => "deny",
        }
    }

    /// The port glyph.
    pub fn glyph(self) -> &'static str {
        match self {
            LinkType::Uses => "\u{25CF}",
            LinkType::Implements => "\u{25C6}",
            LinkType::Reexports => "\u{25B2}",
            LinkType::Deny => "\u{2715}",
        }
    }

    /// Kinds of the rule (`Deny`: every kind).
    pub fn kinds(self) -> Vec<String> {
        match self {
            LinkType::Deny => Vec::new(),
            t => vec![t.name().to_string()],
        }
    }

    pub fn action(self) -> &'static str {
        if self == LinkType::Deny {
            "deny"
        } else {
            "allow"
        }
    }

    /// The type a rule is drawn as: deny, or its first kind (`uses` when it names none).
    pub fn of_rule(kinds: &[String], action: &str) -> LinkType {
        if action == "deny" {
            return LinkType::Deny;
        }
        match kinds.first().map(String::as_str) {
            Some("implements") => LinkType::Implements,
            Some("reexports") => LinkType::Reexports,
            _ => LinkType::Uses,
        }
    }
}

/// The rule spec a port drop adds (no reason yet: the inspector asks for it).
pub fn link_rule(from: &str, to: &str, ty: LinkType, scope: &str) -> RuleSpec {
    RuleSpec {
        from: from.to_string(),
        to: to.to_string(),
        via: Vec::new(),
        kinds: ty.kinds(),
        scope: Some(scope.to_string()),
        action: ty.action().to_string(),
        reason: None,
        help: None,
    }
}

/// `add_rule` of a port drop.
pub fn link_op(file: &str, from: &str, to: &str, ty: LinkType, scope: &str) -> Op {
    Op::AddRule {
        file: file.to_string(),
        rule: link_rule(from, to, ty, scope),
    }
}

/// Whether selector `sel` (`*`, a name, `tag:<t>`) names brick `b`.
pub fn names(sel: &str, b: &BrickView) -> bool {
    match sel.strip_prefix("tag:") {
        _ if sel == "*" => true,
        Some(t) => b.tags.iter().any(|x| x == t),
        None => b.name == sel,
    }
}

/// Scope key `1`/`2`/`3` -> `prod`/`test`/`all`.
pub fn scope_key(key: &str) -> Option<&'static str> {
    match key {
        "1" => Some("prod"),
        "2" => Some("test"),
        "3" => Some("all"),
        _ => None,
    }
}

/// The ops that turn rule `r` (as it is now, `old`) into `new`: a `set_rule` patch when only
/// action, reason, via or help change; remove and add (one version) when its identity (from,
/// to, kinds, scope) changes. An emptied reason / help / via is removed. Empty when unchanged.
pub fn rule_ops(r: &super::RuleRef, old: &RuleSpec, new: &RuleSpec) -> Vec<Op> {
    if old == new {
        return Vec::new();
    }
    if super::rule_key(old) != super::rule_key(new) {
        return vec![
            Op::RemoveRule {
                file: r.file.clone(),
                key: r.key.clone(),
            },
            Op::AddRule {
                file: r.file.clone(),
                rule: new.clone(),
            },
        ];
    }
    let text =
        |a: &Option<String>, b: &Option<String>| (a != b).then(|| b.clone().unwrap_or_default());
    vec![Op::SetRule {
        file: r.file.clone(),
        key: r.key.clone(),
        set: RulePatch {
            action: (old.action != new.action).then(|| new.action.clone()),
            reason: text(&old.reason, &new.reason),
            via: (old.via != new.via).then(|| new.via.clone()),
            help: text(&old.help, &new.help),
        },
    }]
}

/// The ops that point rule `r` (as it is now, `rule`) at `to`. `None` when unchanged.
pub fn retarget_ops(r: &super::RuleRef, rule: &RuleSpec, to: &str) -> Option<Vec<Op>> {
    let new = RuleSpec {
        to: to.to_string(),
        ..rule.clone()
    };
    (rule.to != to).then(|| rule_ops(r, rule, &new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_types_fix_kind_and_action() {
        let r = link_rule("a", "b", LinkType::Implements, "prod");
        assert_eq!(
            (r.kinds, r.action.as_str()),
            (vec!["implements".to_string()], "allow")
        );
        let d = link_rule("a", "tag:io", LinkType::Deny, "all");
        assert!(d.kinds.is_empty());
        assert_eq!(d.action, "deny");
        assert_eq!(LinkType::of_rule(&[], "deny"), LinkType::Deny);
        assert_eq!(
            LinkType::of_rule(&["reexports".into()], "allow"),
            LinkType::Reexports
        );
        assert_eq!(scope_key("2"), Some("test"));
    }

    #[test]
    fn selectors_name_bricks() {
        let b = BrickView {
            name: "api".into(),
            tags: vec!["io".into()],
            ..Default::default()
        };
        assert!(names("*", &b) && names("api", &b) && names("tag:io", &b));
        assert!(!names("ui", &b) && !names("tag:x", &b));
    }

    #[test]
    fn rule_edits_patch_or_replace_by_identity() {
        let old = link_rule("a", "b", LinkType::Uses, "prod");
        let r = super::super::RuleRef::of("brickie.toml", &old);
        assert!(rule_ops(&r, &old, &old).is_empty());
        let why = RuleSpec {
            reason: Some("ports".into()),
            action: "deny".into(),
            ..old.clone()
        };
        let ops = rule_ops(&r, &old, &why);
        let [Op::SetRule { key, set, .. }] = ops.as_slice() else {
            panic!("expected one set_rule");
        };
        assert_eq!(key, &r.key);
        assert_eq!(
            (
                set.action.as_deref(),
                set.reason.as_deref(),
                set.via.clone()
            ),
            (Some("deny"), Some("ports"), None)
        );
        let cleared = rule_ops(&super::super::RuleRef::of("brickie.toml", &why), &why, &old);
        let [Op::SetRule { set, .. }] = cleared.as_slice() else {
            panic!("expected one set_rule");
        };
        assert_eq!(set.reason.as_deref(), Some(""));
        let test = RuleSpec {
            scope: Some("test".into()),
            ..old.clone()
        };
        assert!(matches!(
            rule_ops(&r, &old, &test).as_slice(),
            [Op::RemoveRule { .. }, Op::AddRule { .. }]
        ));
    }

    #[test]
    fn retarget_replaces_the_rule_and_keeps_its_fields() {
        let rule = RuleSpec {
            via: vec!["ports-in".into()],
            reason: Some("why".into()),
            ..link_rule("a", "b", LinkType::Uses, "prod")
        };
        let r = super::super::RuleRef::of("brickie.toml", &rule);
        assert!(retarget_ops(&r, &rule, "b").is_none());
        let ops = retarget_ops(&r, &rule, "tag:io").unwrap();
        let [Op::RemoveRule { key, .. }, Op::AddRule { rule: n, .. }] = ops.as_slice() else {
            panic!("expected remove_rule then add_rule");
        };
        assert_eq!((key.from.as_str(), key.to.as_str()), ("a", "b"));
        assert_eq!(
            (n.to.as_str(), n.reason.clone(), n.via.clone()),
            ("tag:io", rule.reason.clone(), rule.via.clone())
        );
    }
}
