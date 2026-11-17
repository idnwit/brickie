//! The CLI name of a selection, to paste into the coding agent's chat: a brick path
//! (`web/api`), a rule (`rule web -> store (prod)`) or a link (`link web -> store (all)`).

use brickie_web_api::design::{FileSheet, RuleSpec};

/// The path of brick `name` in level `lv` (`root/web`), joined with `/`.
pub fn brick_ref(lv: &str, name: &str) -> String {
    super::brick_path(lv, name).join("/")
}

/// `rule from -> to (scope)` (`link ...` for a link); the `[kinds]` are added only when another
/// row of the same file and table has the same ends and scope, as the CLI needs to tell them
/// apart.
pub fn rule_ref(sheet: Option<&FileSheet>, link: bool, r: &RuleSpec) -> String {
    let scope = r.scope.as_deref().unwrap_or("all");
    let same = |x: &&RuleSpec| {
        x.from == r.from && x.to == r.to && x.scope.as_deref().unwrap_or("all") == scope
    };
    let rows = |s: &FileSheet| {
        if link {
            s.links.clone()
        } else {
            s.rules.clone()
        }
    };
    let twins = sheet.map_or(0, |s| rows(s).iter().filter(same).count());
    let what = if link { "link" } else { "rule" };
    let kinds = match twins > 1 && !r.kinds.is_empty() {
        true => {
            let mut k = r.kinds.clone();
            k.sort();
            format!(" [{}]", k.join(", "))
        }
        false => String::new(),
    };
    format!("{what} {} -> {}{kinds} ({scope})", r.from, r.to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kinds: &[&str], scope: Option<&str>) -> RuleSpec {
        RuleSpec {
            from: "web".into(),
            to: "store".into(),
            kinds: kinds.iter().map(|k| k.to_string()).collect(),
            scope: scope.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn a_brick_ref_is_its_path() {
        assert_eq!(brick_ref("root/web", "api"), "web/api");
        assert_eq!(brick_ref("root", "web"), "web");
    }

    #[test]
    fn a_rule_ref_names_kinds_only_to_tell_twins_apart() {
        let a = rule(&["uses"], None);
        let b = rule(&["implements"], None);
        let one = FileSheet {
            rules: vec![a.clone()],
            ..Default::default()
        };
        assert_eq!(rule_ref(Some(&one), false, &a), "rule web -> store (all)");
        assert_eq!(rule_ref(Some(&one), true, &a), "link web -> store (all)");
        let two = FileSheet {
            rules: vec![a.clone(), b],
            ..Default::default()
        };
        assert_eq!(
            rule_ref(Some(&two), false, &a),
            "rule web -> store [uses] (all)"
        );
        let dev = rule(&["uses"], Some("dev"));
        let other = FileSheet {
            rules: vec![a, dev.clone()],
            ..Default::default()
        };
        assert_eq!(
            rule_ref(Some(&other), false, &dev),
            "rule web -> store (dev)"
        );
    }
}
