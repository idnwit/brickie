//! Whether a stale write may be re-sent on the new head: only when no version between its base
//! and the head touched an identity the write touches (a brick by name, a rule by file and
//! key, a complexity limit by file and `bricks`). A version without ops (a hand edit or an
//! undo) is unknown and touches everything.

use brickie_change::{Op, RuleKey, RuleSpec, Version};

/// A thing an op touches.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ident {
    Brick(String),
    Group(String),
    /// A unit (brick or group) a rule, link or brick.group names: clashes with changes to that
    /// brick or group, not with other references naming it.
    Named(String),
    /// A `[[rule]]` (`false`) or `[[link]]` (`true`) by file and key.
    Rule(String, bool, RuleKey),
    Limit(String, String),
    /// File-level settings: exports, includes, the file itself.
    File(String),
}

impl Ident {
    /// Two identities clash when they name the same thing. A key without `kinds` stands for
    /// any kinds; a key without `scope` means `all`.
    fn clashes(&self, other: &Ident) -> bool {
        match (self, other) {
            (Ident::Rule(fa, la, a), Ident::Rule(fb, lb, b)) => {
                fa == fb
                    && la == lb
                    && a.from == b.from
                    && a.to == b.to
                    && same_set(&a.kinds, &b.kinds)
                    && a.scope.as_deref().unwrap_or("all") == b.scope.as_deref().unwrap_or("all")
            }
            (Ident::Group(a), Ident::Brick(b))
            | (Ident::Brick(a), Ident::Group(b))
            | (Ident::Named(a), Ident::Brick(b))
            | (Ident::Brick(a), Ident::Named(b))
            | (Ident::Named(a), Ident::Group(b))
            | (Ident::Group(a), Ident::Named(b)) => a == b,
            (Ident::Named(_), Ident::Named(_)) => false,
            _ => self == other,
        }
    }

    fn text(&self) -> String {
        match self {
            Ident::Brick(n) => format!("brick {n}"),
            Ident::Named(n) => n.clone(),
            Ident::Group(n) => format!("group {n}"),
            Ident::Rule(_, link, k) => {
                let kinds = match &k.kinds {
                    Some(k) if !k.is_empty() => format!(" [{}]", k.join(",")),
                    _ => String::new(),
                };
                let scope = k
                    .scope
                    .as_deref()
                    .map_or(String::new(), |s| format!(" ({s})"));
                let what = if *link { "link" } else { "rule" };
                format!("{what} {} -> {}{kinds}{scope}", k.from, k.to)
            }
            Ident::Limit(_, b) => format!("limits of {b}"),
            Ident::File(f) => format!("settings of {f}"),
        }
    }
}

fn same_set(a: &Option<Vec<String>>, b: &Option<Vec<String>>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => {
            let (mut x, mut y) = (x.clone(), y.clone());
            x.sort();
            y.sort();
            x == y
        }
        _ => true,
    }
}

fn spec_key(r: &RuleSpec) -> RuleKey {
    RuleKey {
        from: r.from.clone(),
        to: r.to.clone(),
        kinds: (!r.kinds.is_empty()).then(|| r.kinds.clone()),
        scope: r.scope.clone(),
    }
}

fn limit(file: &str, bricks: &Option<String>) -> Ident {
    Ident::Limit(file.into(), bricks.clone().unwrap_or_else(|| "*".into()))
}

fn limit_idents(file: &str, bricks: &Option<String>) -> Vec<Ident> {
    let mut out = vec![limit(file, bricks)];
    if let Some(b) = bricks.as_deref()
        && !b.is_empty()
        && b != "*"
        && !b.starts_with("tag:")
    {
        out.push(Ident::Named(b.to_string()));
    }
    out
}

/// A rule or link also depends on the units it names (a link's `composite/child` names both the
/// composite and the exported child brick or group): renaming or removing either stales the write.
fn rule_idents(file: &str, link: bool, key: RuleKey) -> Vec<Ident> {
    let mut out = Vec::with_capacity(4);
    let mut push_end = |s: &str| {
        if let Some((parent, child)) = s.split_once('/') {
            out.push(Ident::Named(parent.to_string()));
            if !child.is_empty() {
                out.push(Ident::Named(child.to_string()));
            }
        } else {
            out.push(Ident::Named(s.to_string()));
        }
    };
    push_end(&key.from);
    push_end(&key.to);
    out.push(Ident::Rule(file.into(), link, key));
    out
}

fn touched(op: &Op) -> Vec<Ident> {
    let brick = |n: &String| Ident::Brick(n.clone());
    let group = |n: &String| Ident::Group(n.clone());
    match op {
        Op::AddBrick { brick: b, .. } => {
            let mut out = vec![Ident::Brick(b.name.clone())];
            if let Some(g) = b.group.as_deref().filter(|g| !g.is_empty()) {
                out.push(Ident::Named(g.to_string()));
            }
            out
        }
        Op::SetBrick { name, group: g, .. } => {
            let mut out = vec![brick(name)];
            if let Some(g) = g.as_deref().filter(|g| !g.is_empty()) {
                out.push(Ident::Named(g.to_string()));
            }
            out
        }
        Op::RemoveBrick { name, .. } | Op::MakeComposite { name, .. } | Op::Unnest { name, .. } => {
            vec![brick(name)]
        }
        Op::RenameBrick { from, to, .. } => vec![brick(from), brick(to)],
        Op::MoveBrick { name, into, .. } => std::iter::once(brick(name))
            .chain(into.iter().map(brick))
            .collect(),
        Op::AddGroup { group: g, .. } => vec![group(&g.name)],
        Op::SetGroup { name, .. } | Op::RemoveGroup { name, .. } => vec![group(name)],
        Op::RenameGroup { name, new_name, .. } => vec![group(name), group(new_name)],
        Op::AddRule { file, rule } => rule_idents(file, false, spec_key(rule)),
        Op::RemoveRule { file, key } | Op::SetRule { file, key, .. } => {
            rule_idents(file, false, key.clone())
        }
        Op::AddLink { file, link } => rule_idents(file, true, spec_key(link)),
        Op::RemoveLink { file, key } | Op::SetLink { file, key, .. } => {
            rule_idents(file, true, key.clone())
        }
        Op::AddComplexity { file, rule } => limit_idents(file, &rule.bricks),
        Op::RemoveComplexity { file, bricks } | Op::SetComplexity { file, bricks, .. } => {
            limit_idents(file, bricks)
        }
        Op::AddExport { file, brick } | Op::RemoveExport { file, brick } => {
            vec![Ident::File(file.clone()), Ident::Named(brick.clone())]
        }
        Op::AddInclude { file, .. } | Op::RemoveInclude { file, .. } | Op::CreateFile { file } => {
            vec![Ident::File(file.clone())]
        }
    }
}

/// `<author> v<N> changed <identity>` for the first version after `base` that touched what
/// `ops` touch; `None` when the write is safe to re-send.
pub fn first_conflict(log: &[Version], base: u64, ops: &[Op]) -> Option<String> {
    let mine: Vec<Ident> = ops.iter().flat_map(touched).collect();
    for v in log.iter().filter(|v| v.v > base) {
        let who = format!("{} v{}", v.by.name(), v.v);
        if v.ops.is_empty() {
            return Some(format!("{who} changed the design"));
        }
        let mut theirs: Vec<Ident> = v.ops.iter().flat_map(touched).collect();
        if v.ops.iter().any(|op| matches!(op, Op::RenameGroup { .. })) {
            theirs.extend(v.files.iter().cloned().map(Ident::File));
        }
        for t in theirs {
            if mine.iter().any(|m| m.clashes(&t)) {
                return Some(format!("{who} changed {}", t.text()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use brickie_change::{BrickSpec, By};

    fn ver(v: u64, by: By, ops: Vec<Op>) -> Version {
        Version {
            v,
            track: Default::default(),
            forked_from: None,
            reverts: None,
            hash: String::new(),
            prev: None,
            by,
            at: 0,
            note: None,
            ops,
            diff: Vec::new(),
            files: Vec::new(),
            removed: Vec::new(),
            full: false,
        }
    }

    fn add(name: &str) -> Op {
        Op::AddBrick {
            file: "brickie.toml".into(),
            brick: BrickSpec {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    fn rule(to: &str, scope: Option<&str>) -> Op {
        Op::RemoveRule {
            file: "brickie.toml".into(),
            key: RuleKey {
                from: "web".into(),
                to: to.into(),
                kinds: None,
                scope: scope.map(Into::into),
            },
        }
    }

    #[test]
    fn disjoint_versions_allow_a_resend() {
        let log = vec![ver(2, By::Agent, vec![add("cache")])];
        assert_eq!(first_conflict(&log, 1, &[add("other")]), None);
        assert_eq!(
            first_conflict(&log, 2, &[add("cache")]),
            None,
            "before base"
        );
    }

    #[test]
    fn the_same_brick_conflicts_and_names_author_and_identity() {
        let log = vec![
            ver(2, By::Human, vec![add("x")]),
            ver(3, By::Agent, vec![add("cache")]),
        ];
        assert_eq!(
            first_conflict(&log, 1, &[add("cache")]).as_deref(),
            Some("agent v3 changed brick cache")
        );
    }

    #[test]
    fn rules_match_by_file_and_key() {
        let log = vec![ver(2, By::Agent, vec![rule("store", Some("all"))])];
        assert!(first_conflict(&log, 1, &[rule("store", None)]).is_some());
        assert_eq!(
            first_conflict(&log, 1, &[rule("store", Some("prod"))]),
            None
        );
        assert_eq!(first_conflict(&log, 1, &[rule("db", None)]), None);
        assert_eq!(
            first_conflict(&log, 1, &[rule("store", None)]).as_deref(),
            Some("agent v2 changed rule web -> store (all)")
        );
        // A link with the same key is a different thing.
        let link = Op::RemoveLink {
            file: "brickie.toml".into(),
            key: RuleKey {
                from: "web".into(),
                to: "store".into(),
                kinds: None,
                scope: None,
            },
        };
        assert_eq!(first_conflict(&log, 1, &[link]), None);
    }

    #[test]
    fn a_rule_conflicts_with_a_rename_or_removal_of_a_brick_it_names() {
        let rm = Op::RemoveBrick {
            file: "brickie.toml".into(),
            name: "store".into(),
        };
        let log = vec![ver(2, By::Human, vec![rm])];
        let add_rule = Op::AddRule {
            file: "brickie.toml".into(),
            rule: RuleSpec {
                from: "web".into(),
                to: "store".into(),
                ..Default::default()
            },
        };
        assert_eq!(
            first_conflict(&log, 1, &[add_rule]).as_deref(),
            Some("human v2 changed brick store")
        );
    }

    #[test]
    fn limits_match_by_bricks_and_opless_versions_conflict() {
        let lim = |b: Option<&str>| Op::RemoveComplexity {
            file: "brickie.toml".into(),
            bricks: b.map(Into::into),
        };
        let log = vec![ver(2, By::Agent, vec![lim(Some("web"))])];
        assert!(first_conflict(&log, 1, &[lim(Some("web"))]).is_some());
        assert_eq!(first_conflict(&log, 1, &[lim(None)]), None);
        let hand = vec![ver(2, By::External, Vec::new())];
        assert_eq!(
            first_conflict(&hand, 1, &[add("a")]).as_deref(),
            Some("external v2 changed the design")
        );
    }

    #[test]
    fn group_conflicts_cover_namespace_and_membership() {
        use brickie_change::GroupSpec;
        let add_grp = |name: &str| Op::AddGroup {
            file: "brickie.toml".into(),
            group: GroupSpec {
                name: name.into(),
                ..Default::default()
            },
        };
        let rm_grp = |name: &str| Op::RemoveGroup {
            file: "brickie.toml".into(),
            name: name.into(),
        };
        let ren_grp = |name: &str, new_name: &str| Op::RenameGroup {
            file: "brickie.toml".into(),
            name: name.into(),
            new_name: new_name.into(),
        };
        let set_mem = |brick: &str, grp: &str| Op::SetBrick {
            file: "brickie.toml".into(),
            name: brick.into(),
            desc: None,
            tags: None,
            members: None,
            path: None,
            independent: None,
            group: Some(grp.into()),
        };
        let add_mem = |brick: &str, grp: &str| Op::AddBrick {
            file: "brickie.toml".into(),
            brick: BrickSpec {
                name: brick.into(),
                group: Some(grp.into()),
                ..Default::default()
            },
        };

        // Group and brick share one namespace.
        let log = vec![ver(2, By::Agent, vec![add_grp("services")])];
        assert!(first_conflict(&log, 1, &[add("services")]).is_some());
        assert_eq!(first_conflict(&log, 1, &[add("other")]), None);

        // Rule naming a group clashes with RemoveGroup / RenameGroup.
        let log_rm = vec![ver(2, By::Human, vec![rm_grp("services")])];
        assert!(first_conflict(&log_rm, 1, &[rule("services", None)]).is_some());

        // SetBrick.group and AddBrick.group clash with RenameGroup/RemoveGroup.
        assert!(first_conflict(&log_rm, 1, &[set_mem("auth", "services")]).is_some());
        assert!(first_conflict(&log_rm, 1, &[add_mem("notify", "services")]).is_some());
        let log_ren = vec![ver(2, By::Human, vec![ren_grp("services", "svc")])];
        assert!(first_conflict(&log_ren, 1, &[set_mem("auth", "services")]).is_some());

        // Two different bricks joining the same group do NOT conflict with each other.
        let log_join = vec![ver(2, By::Agent, vec![set_mem("auth", "services")])];
        assert_eq!(
            first_conflict(&log_join, 1, &[set_mem("billing", "services")]),
            None
        );

        // Link to `composite/group` clashes with renaming or removing `group`.
        let link_via = Op::AddLink {
            file: "brickie.toml".into(),
            link: RuleSpec {
                from: "cli".into(),
                to: "web/services".into(),
                ..Default::default()
            },
        };
        assert!(first_conflict(&log_rm, 1, std::slice::from_ref(&link_via)).is_some());
        assert!(first_conflict(&log_ren, 1, &[link_via]).is_some());

        // Export and concrete complexity ops clash with renaming or removing the named unit.
        let add_exp = Op::AddExport {
            file: "web/brickie.toml".into(),
            brick: "services".into(),
        };
        assert!(first_conflict(&log_rm, 1, &[add_exp]).is_some());
        let rm_cx = Op::RemoveComplexity {
            file: "brickie.toml".into(),
            bricks: Some("services".into()),
        };
        assert!(first_conflict(&log_rm, 1, &[rm_cx]).is_some());
        let tag_cx = Op::RemoveComplexity {
            file: "brickie.toml".into(),
            bricks: Some("tag:services".into()),
        };
        assert_eq!(first_conflict(&log_rm, 1, &[tag_cx]), None);

        // Multi-file RenameGroup clashes its rewritten files against File-level ops.
        let mut v_ren = ver(2, By::Human, vec![ren_grp("services", "svc")]);
        v_ren.files = vec!["brickie.toml".into(), "parent/brickie.toml".into()];
        let inc = Op::AddInclude {
            file: "parent/brickie.toml".into(),
            path: "extra.toml".into(),
        };
        assert_eq!(
            first_conflict(&[v_ren], 1, &[inc]).as_deref(),
            Some("human v2 changed settings of parent/brickie.toml")
        );

        // Ident::Named formats neutrally without a "brick" prefix.
        let log_link = vec![ver(2, By::Agent, vec![rule("services", None)])];
        assert_eq!(
            first_conflict(&log_link, 1, &[rm_grp("services")]).as_deref(),
            Some("agent v2 changed services")
        );
    }
}
