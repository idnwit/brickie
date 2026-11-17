//! Typed links: the three port types of a brick (a dependency kind), the `[[link]]` a drop
//! adds, which bricks a rule selector or a link end names, and the ops that edit a rule or link.

use brickie_web_api::design::{Op, RulePatch, RuleSpec};
use brickie_web_api::{BrickView, LevelLink};

/// The type of a link port: it fixes the link's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Uses,
    Implements,
    Reexports,
}

pub const LINK_TYPES: [LinkType; 3] = [LinkType::Uses, LinkType::Implements, LinkType::Reexports];

pub const SCOPES: [&str; 3] = ["prod", "test", "all"];

/// The reason a link drawn on the canvas starts with (`reason` is required; the inspector
/// edits it).
pub const DRAWN_REASON: &str = "drawn in the editor";

impl LinkType {
    pub fn name(self) -> &'static str {
        match self {
            LinkType::Uses => "uses",
            LinkType::Implements => "implements",
            LinkType::Reexports => "reexports",
        }
    }

    /// The port glyph.
    pub fn glyph(self) -> &'static str {
        match self {
            LinkType::Uses => "\u{25CF}",
            LinkType::Implements => "\u{25C6}",
            LinkType::Reexports => "\u{25B2}",
        }
    }

    pub fn kinds(self) -> Vec<String> {
        vec![self.name().to_string()]
    }

    /// The type a rule or link is drawn as: its first kind (`uses` when it names none).
    pub fn of_kinds(kinds: &[String]) -> LinkType {
        match kinds.first().map(String::as_str) {
            Some("implements") => LinkType::Implements,
            Some("reexports") => LinkType::Reexports,
            _ => LinkType::Uses,
        }
    }
}

/// The spec a port drop adds (the reason starts as [`DRAWN_REASON`]).
pub fn link_rule(from: &str, to: &str, ty: LinkType, scope: &str) -> RuleSpec {
    RuleSpec {
        from: from.to_string(),
        to: to.to_string(),
        kinds: ty.kinds(),
        scope: Some(scope.to_string()),
        reason: DRAWN_REASON.to_string(),
    }
}

/// `add_link` of a port drop.
pub fn link_op(file: &str, from: &str, to: &str, ty: LinkType, scope: &str) -> Op {
    Op::AddLink {
        file: file.to_string(),
        link: link_rule(from, to, ty, scope),
    }
}

/// Whether the canvas draws link `e`: every link in Design mode, only planned ones (no code
/// uses them yet) in view mode, where the measured edges show the rest.
pub fn drawn(e: &LevelLink, editing: bool) -> bool {
    editing || e.planned
}

/// The brick a link end names: `composite/child` lands on the composite.
pub fn link_brick(end: &str) -> &str {
    end.split('/').next().unwrap_or(end)
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

/// The ops that turn rule or link `r` (as it is now, `old`) into `new`: a `set_rule` /
/// `set_link` patch when only the reason changes; remove and add (one version) when its
/// identity (from, to, kinds, scope) changes. Empty when unchanged.
pub fn rule_ops(r: &super::RuleRef, old: &RuleSpec, new: &RuleSpec) -> Vec<Op> {
    if old == new {
        return Vec::new();
    }
    let (file, key) = (r.file.clone(), r.key.clone());
    if super::rule_key(old) != super::rule_key(new) {
        let spec = new.clone();
        return if r.link {
            vec![
                Op::RemoveLink {
                    file: file.clone(),
                    key,
                },
                Op::AddLink { file, link: spec },
            ]
        } else {
            vec![
                Op::RemoveRule {
                    file: file.clone(),
                    key,
                },
                Op::AddRule { file, rule: spec },
            ]
        };
    }
    let set = RulePatch {
        reason: Some(new.reason.clone()),
    };
    if r.link {
        vec![Op::SetLink { file, key, set }]
    } else {
        vec![Op::SetRule { file, key, set }]
    }
}

/// The ops that point rule or link `r` (as it is now, `rule`) at `to`. `None` when unchanged.
pub fn retarget_ops(r: &super::RuleRef, rule: &RuleSpec, to: &str) -> Option<Vec<Op>> {
    let new = RuleSpec {
        to: to.to_string(),
        ..rule.clone()
    };
    (rule.to != to).then(|| rule_ops(r, rule, &new))
}

#[cfg(test)]
mod tests {
    use super::super::RuleRef;
    use super::*;

    #[test]
    fn port_types_fix_the_kind() {
        let r = link_rule("a", "b", LinkType::Implements, "prod");
        assert_eq!(r.kinds, vec!["implements".to_string()]);
        assert_eq!(r.reason, DRAWN_REASON);
        assert_eq!(
            LinkType::of_kinds(&["reexports".into()]),
            LinkType::Reexports
        );
        assert_eq!(LinkType::of_kinds(&[]), LinkType::Uses);
        assert_eq!(scope_key("2"), Some("test"));
        assert_eq!(link_brick("web/api"), "web");
        assert!(matches!(
            link_op("f", "a", "b", LinkType::Uses, "all"),
            Op::AddLink { .. }
        ));
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
    fn edits_patch_or_replace_by_identity() {
        let old = link_rule("a", "b", LinkType::Uses, "prod");
        let r = RuleRef::link("brickie.toml", &old);
        assert!(rule_ops(&r, &old, &old).is_empty());
        let why = RuleSpec {
            reason: "ports".into(),
            ..old.clone()
        };
        let ops = rule_ops(&r, &old, &why);
        let [Op::SetLink { key, set, .. }] = ops.as_slice() else {
            panic!("expected one set_link");
        };
        assert_eq!((key, set.reason.as_deref()), (&r.key, Some("ports")));
        let test = RuleSpec {
            scope: Some("test".into()),
            ..old.clone()
        };
        assert!(matches!(
            rule_ops(&r, &old, &test).as_slice(),
            [Op::RemoveLink { .. }, Op::AddLink { .. }]
        ));
        let rule = RuleRef::of("brickie.toml", &old);
        assert!(matches!(
            rule_ops(&rule, &old, &why).as_slice(),
            [Op::SetRule { .. }]
        ));
    }

    #[test]
    fn retarget_replaces_and_keeps_the_fields() {
        let rule = link_rule("a", "b", LinkType::Uses, "prod");
        let r = RuleRef::link("brickie.toml", &rule);
        assert!(retarget_ops(&r, &rule, "b").is_none());
        let ops = retarget_ops(&r, &rule, "c").unwrap();
        let [Op::RemoveLink { key, .. }, Op::AddLink { link: n, .. }] = ops.as_slice() else {
            panic!("expected remove_link then add_link");
        };
        assert_eq!(
            (key.to.as_str(), n.to.as_str(), n.reason.as_str()),
            ("b", "c", DRAWN_REASON)
        );
    }
}
